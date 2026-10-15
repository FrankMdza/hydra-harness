# 130 - The human's docket of 2026-10-15: the red Linux job, builds 316 and 326, the reviews of 320 and 321, maps 126 and 127, and the publication list

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, at the human's request. Every item the human still has to
decide, in one ticket, each written three ways before any recommendation: the problem or the
objective, how it is being solved, and the test that proves it solved. Grilling: the human
answers; the panel recommends and the session synthesizes. Decisions land in the originating
ticket (316, 326, 126, 127) and this one points at them.

## Question

Read `.scratch/hydra/research/130-panel-readings.md` first. It carries the items and the triad
for each. Then, per item, say which way you recommend and why, what in the readings you checked
against the repo and found true or false, and what test you would demand before calling the item
closed, if the one in the readings is not enough.

1. The Linux job (item 1): narrow the job to what is true today, leave it red with eight tickets,
   or fix the eight groups before the flip. And for the four groups that are decisions rather than
   test fixes: whose identity reap commits with when none is configured (c), which answer to a NUL
   `cwd` is right (d), and whether the CPython ports match CPython per platform or accept a
   one-ulp tolerance (g).
2. Build 316 (item 2): schema version 2 with `sandbox: {kind, sha}`, or additive `sandbox_kind`.
3. Build 326 (item 3): commit the nested repository's files and drop the gitlink, or refuse by
   name.
4. Build 321 (item 4): accept "written by the hook, not tamper-proof" as the evidence's standing in
   the contract, or sign the sidecar's lines.
5. Build 320 (item 5): leave the three facts in text, or spend four drift probes and add JSON
   fields.
6. Map 126 (item 6): the two questions of that ticket.
7. Map 127 (item 7): the three questions of that ticket.
8. The publication list (item 8): the order, anything missing, anything in it that should not be
   done.
9. The unopened tickets (item 9): rank them, or say which not to open.

## Panel

Four seats sat on 2026-10-15, all under `standard`, none ran cargo: Opus 5.5 (`research/130-seat-opus.md`), Grok 4.7 (`130-seat-grok.md`), Gemini 3.8 Flash (`130-seat-gemini.md`) and GPT 6.1 Sol (`130-seat-gpt.md`). Gemini's and GPT's worktrees were seeded with the earlier memos already copied into master, so Grok is the one fully independent second reading. The session's synthesis with a recommendation per question: `research/130-synthesis.md`. Its first finding: eight claims in the readings were wrong and all eight corrections checked out, among them that group f of the Linux job is a test bug visible on the Mac, that the next release is 0.3.0 and not 0.1.0, and that `.github/` is not protected ground. The seats split two to two on 326 and 127; the session recommends refusing by name on 326 and waiting on 127. Nothing under Answer is written; the human answers.

## Answer

Resolved 2026-10-15 by the human, who took the session's recommendation from the panel 130 synthesis as it stands ("me gustan tus recomendaciones", "sigue tus recomendaciones para el resto"). On question 1 the human added one instruction: test and fix the Linux failures on a real Linux, not from the Mac ("no podrías correr algún server que te deje correrlo para que hagas pruebas y corrijas de verdad"). The session runs them in a Docker container on this machine (`rust:1-bookworm`, linux/aarch64, 4 GB) and the `ubuntu-latest` job stays the x86-64 check.

1. Linux job: fix groups a, b, f and h (test-only), d as an explicit `hydra/bad-payload` refusal, c by pinning `user.name` in the fixture plus one measurement of git's own guess on a clean macOS `HOME`; e and g skipped by test name until measured, the README naming the skipped tests. Build 331, the session's own, verified in the container.
2. Build 316: additive, `host` and `sandbox_kind` together, schema version stays 1, `absent` means unknown. The ticket's What is rewritten; the held worktree `01a13dff-4745` is reaped without applying.
3. Build 326: refuse by name before any commit. Decision written on the ticket.
4. Build 321: the standing in contract 7.7 is accepted as written; build 332 adds the forged-row test, the damaged-sidecar test and the walk's corroboration check.
5. Build 320: the facts stay in text; closed as it stands. If a consumer ever appears, the fields go on the 321 sidecar row.
6. Map 126: see its Answer.
7. Map 127: see its Answer.
8. Publication: the order in the synthesis. `gitleaks` installed and run over 1034 commits the same day; the `security` workflow's cause confirmed from the reusable workflow's own file (its job asks `security-events: write`, the caller grants none); 0.3.0; the three untracked files ignored; the session writes the one sentence in `config/README.md`; `git gc` last.
9. Tickets opened: 333 (the five-second cockpit test), 334 (a call the provider rejected before the hook is not `Ungated`), 335 (`hydra show --verdict` reading the puppets goal logs, defined first). Not opened: JSON refusal fields, OpenCode.
