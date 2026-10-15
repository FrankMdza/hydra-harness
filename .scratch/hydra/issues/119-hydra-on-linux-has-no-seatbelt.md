# 119 - On a Linux VM the gate is hook-only: what wraps a launch there

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-12, by the session, from the cloud research of the same day
(`~/cerebro/research/cloud-vm.md`, section 6). The human wants his sessions on an always-on VM
so the laptop can close; the cheapest workable boxes are Linux, x86 or Arm.

## Question

`src/sandbox.rs` line 46: `wraps_launches()` is `cfg!(target_os = "macos")`. On Linux Hydra
launches with the hook config and no OS sandbox, and `run_start.sandbox` is empty. The seatbelt
is what makes `cat ~/.claude/x` refusable under the third policy (`CLAUDE.md`, "Nothing here is
enforced"); without it the VM population gets the gate of 2026-09-14 before the terminal rule.

1. Is hook-only gating acceptable for runs on the VM, as a named population with its own
   `policy_sha` and a record field that says no sandbox wrapped it? Or do recorded runs stay on
   the Mac until Linux has a wrapper?
2. If a wrapper: bubblewrap (user namespaces, needs them enabled on the distro) or Landlock
   (kernel 5.13 and up, no setuid, path-based like the seatbelt's rules). Which matches the
   seatbelt profile `sandbox.rs` composes closer, and what in `spawn_config.rs` row 4 changes?
3. The four CLIs ship Linux arm64 (verified in the report, section 5). Does the provider drift
   rule (re-read versions every session) get a per-OS row, or is one version table enough?
4. The records and `~/hydra/runs/` move with the sessions, one-way backup to the laptop. Does
   `hydra doctor --walk` and Rungs need anything for a corpus written on two machines, or is
   `base_tree` enough to tell them apart?

## Answer

Resolved 2026-10-15 by the human, who took the panel's recommendation ("sigo con las
decisiones del panel"). Before any of it: Hydra has to compile on Linux, which it does not today
(`src/procinfo.rs` is Darwin-only with no cfg gate).

1. Recorded runs under `standard` and `permissive` stay on the Mac until Linux has a wrapper. The
   population label is a `host` object on `run_start` (os, arch, machine) and a `kind` beside
   `sandbox`'s sha, not a second policy set: `policy_sha` hashes the same bytes on both machines
   at one commit.
2. Bubblewrap, not Landlock. Row 4 stays the protected writer and writes a bwrap argv instead of
   SBPL, composed from the same `Spawn` in the same order as `compose()`, covering every launch
   path including `interactive.rs` and `evolve.rs`'s admission, with no hook-only fallback. Codex's
   nesting (`danger-full-access` under a wrapper, `src/provider/codex.rs` lines 82 to 86) is
   decided before `wraps_launches()` turns true on Linux, and the five `inside_seatbelt()` sites
   that five human-only commands key on get a Linux witness.
3. One drift table keyed by provider, OS and arch, every row carrying its date and the probe that
   measured it; `hydra doctor` on a platform without a row says so until `tests/drift.rs` has run
   there.
4. `base_tree` names content, not a host; `run_start.host` tells the machines apart. The VM's
   backup lives in its own root, read with `HYDRA_ROOT`, so `live.rs` never reads a VM record as
   abandoned and a bare `hydra prune` never moves one. Whether the reader may take the layout off
   the record for a corpus written on two machines (contract 7.7) goes to map 126.

Tickets: builds 315 (Linux compile), 316 (`run_start.host` and `sandbox.kind`), 317 (the drift
table), 318 (the bubblewrap backend, after 315 and 316); map 126.

## Panel

Three seats sat on 2026-10-13, Opus 5.5, Gemini 3.8 Flash and GPT Astra; no Grok seat while Cursor is on the free plan. Memos: `research/119-seat-opus.md`, `119-seat-gemini.md`, `119-seat-gpt.md`. The session's synthesis with a recommendation per question: `research/119-synthesis.md`. One finding comes before the questions: Hydra does not build on Linux today (`src/procinfo.rs` is Darwin-only with no cfg gate). Nothing under Answer is written; the human answers.
