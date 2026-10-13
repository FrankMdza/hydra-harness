# Map 119, the session's synthesis for the human

Three seats sat on 2026-10-13: Opus 5.5 on `claude` (`119-seat-opus.md`), Gemini 3.8 Flash on
`agy` (`119-seat-gemini.md`), GPT Astra on `codex` (`119-seat-gpt.md`). No Grok seat: Cursor is on
the free plan. Every seat worked inside a Hydra worktree on the Mac and none could read
`~/cerebro/research/cloud-vm.md`, so the ticket's claims about the VM and the four CLIs' Linux
builds are taken as stated by all three. The session checked against the repo every claim that
counts below; where a seat's claim was not checked, it says so.

## A finding before the questions

Opus and GPT both found it, and the session confirmed it: Hydra has never been built for Linux and
does not build there as it is. `src/procinfo.rs` line 24 calls `sysctl` with
`libc::KERN_PROC_PID`, a Darwin constant, and the module has no `cfg(target_os)` gate; the only
`cfg` in the file is `#[cfg(test)]`. `src/lib.rs` line 44 declares it for every target, and
`live.rs`, `watch.rs` and `interactive.rs` call it, so every liveness answer goes through it.
Whatever the human decides on the four questions, the first build ticket is a Linux `cargo build`
with a `/proc/<pid>/stat` start-time reader behind `cfg(target_os = "linux")`. Gemini did not see
this and assumed a Linux binary exists.

## Where the three agree

- **Question 1.** Recorded runs under `standard` and `permissive` stay on the Mac until Linux
  has a wrapper. `policy_sha` is the wrong label for a population: it hashes `policies/` and the
  floor, the same bytes on both machines at one commit (GPT read `src/policy/mod.rs` line 585).
  `sandbox: null` already says no wrapper ran, and it is ambiguous, so the label is a new field
  on `run_start`, not a second policy set.
- **Question 2.** Bubblewrap, not Landlock. The seatbelt profile `compose()` writes is allow by
  default, deny the parent, allow the child back, last match wins; bwrap's mount order has the
  same last-wins rule and most of the twelve rule kinds translate line for line, while Landlock
  only grants and cannot spell "deny `~/hydra/runs` under an allowed `HOME`". Row 4 keeps its
  role as the protected writer; what changes is which profile it writes and that the record's
  `sandbox` needs a kind beside the sha, because a sha does not say whether it hashed SBPL or a
  bwrap argv. GPT adds that the wrapper must cover every launch path, `interactive.rs` and
  `evolve.rs`'s admission included, with no hook-only fallback.
- **Question 4.** `base_tree` cannot tell machines apart because it names content, not a host.
  All three want a `host` or origin object on `run_start` (os, arch, machine), under contract
  section 15's rule for a new field, and all three warn that a VM record copied into
  `~/hydra/runs/` on the laptop is read as `abandoned` by `live.rs` and would be moved by a bare
  `hydra prune`. Opus and GPT want the VM's backup in its own root, read with `HYDRA_ROOT`.

## Where they split, and the recommendation

1. **Question 3, the drift table.** Opus: one table with an OS column saying where each row
   was measured; do not split rows until a vendor's version strings differ by platform. Gemini:
   per-OS rows now, because vendors ship separate binaries with separate cadences. GPT: key the
   table by provider, OS and arch, with the probe results beside the version. These are not
   three votes against each other: Gemini's claim that cadences differ is not measured by anyone
   (no seat ran the four `--version` calls, and `MEASURED_VERSIONS` in `src/provider/mod.rs`
   lines 29 to 34 has four rows and no OS), and Opus's and GPT's shapes are the same table with
   one or two more columns. Recommendation: GPT's shape, one table keyed by provider, OS and
   arch, every row carrying the date and the drift probe that measured it, and `hydra doctor`
   on the VM saying "unmeasured on linux" until `tests/drift.rs` has run there.
2. **What hook-only loses, beyond the floor's misses.** Opus found what the ticket does not
   name: `inside_seatbelt()` is the one fact about ancestry a run cannot shed, and it returns
   `false` on every non-macOS target. The session confirmed the five production sites that lean
   on it: `src/stop.rs` line 73 (`hydra stop` from inside a run), `src/approve.rs` lines 85 and
   147 (`hydra approve` and `hydra deny`), `src/face.rs` line 374 (delegation from inside a
   run), `src/run/mod.rs` line 295 (`HYDRA_ROOT` moved from inside a run), plus `evolve.rs`
   line 2009. On Linux under `standard` a run could list pending seat requests and approve its
   own, which map 82 gave to the human alone. No policy pattern catches these commands. This is
   not a split, the other seats did not look there, but it changes question 1's weight: the
   wrapper is not only defence behind the gate, it is what five human-only commands key on.
3. **Codex nesting.** Opus and GPT both read `src/provider/codex.rs` lines 82 to 86: under a
   wrapper Hydra hands `codex` `danger-full-access`, because a second sandbox cannot nest in the
   seatbelt; on Linux today it gets `workspace-write`. Making `wraps_launches()` true on Linux
   would switch Codex to full access by accident unless the bwrap backend decides nesting first.
   Gemini did not raise it. Not a vote, a trap for the build ticket.
4. **Replay of a Linux record on the Mac.** Gemini claims `gate.py`'s path check fails because
   `os.path.realpath` on macOS turns `/home/ubuntu/x` into `/System/Volumes/Data/home/ubuntu/x`.
   The session ran that call: it does. Opus reads the same situation differently: the reader
   compiles a gate with the laptop's paths, `GateMark::of` files it `Foreign`, and the walk
   claims no leak and re-evaluates nothing. Both are consistent with the code; which one a
   given record hits depends on whether its `gate_sha` matches. Either way, a pooled corpus
   needs the reader to take the layout off the record, which contract 7.7 forbids today. That is
   a map question, not a build.

Fable was not called as judge: the one split, question 3, is a column count and the
recommendation above subsumes two of the three memos.

## What would become tickets if the human agrees

- Build: Hydra compiles on Linux; `procinfo.rs` gains a `/proc/<pid>/stat` reader behind
  `cfg(target_os = "linux")`, CI gains an `ubuntu-latest` build job (compile and the
  non-seatbelt tests only).
- Build: `run_start.host` (os, arch, machine name) and a `kind` beside `sandbox`'s sha, both
  under contract section 15's rule; `hydra doctor --walk` reports the host it is reading.
- Build, after the first two: a bubblewrap backend composed from the same `Spawn` in the same
  order as `compose()`, with the `TIOCSTI` rule as a sysctl `hydra doctor` checks, Codex's
  nesting decided before `wraps_launches()` turns true, and the five `inside_seatbelt()` sites
  given a Linux witness (a mount namespace cannot be shed).
- Build: `MEASURED_VERSIONS` keyed by provider, OS and arch with date and probe.
- Map: whether the reader may take the layout off the record for a corpus written on two
  machines (contract 7.7), and whether Rungs splits evidence by enforcement population.
- Not a ticket yet: the VM's backup in its own root, read with `HYDRA_ROOT`, is a one-line
  rule for the human's sync, not code.
