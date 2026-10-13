# 119 - On a Linux VM the gate is hook-only: what wraps a launch there

Type: grilling
Status: open
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

## Panel

Three seats sat on 2026-10-13, Opus 5.5, Gemini 3.8 Flash and GPT Astra; no Grok seat while Cursor is on the free plan. Memos: `research/119-seat-opus.md`, `119-seat-gemini.md`, `119-seat-gpt.md`. The session's synthesis with a recommendation per question: `research/119-synthesis.md`. One finding comes before the questions: Hydra does not build on Linux today (`src/procinfo.rs` is Darwin-only with no cfg gate). Nothing under Answer is written; the human answers.
