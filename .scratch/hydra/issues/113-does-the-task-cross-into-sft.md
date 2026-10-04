# 113 - Does the task cross into sft.jsonl

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-04, by the session, from the first training round (handoff queue item 12).

## Question

Contract 16.1 maps `run_start` to nothing in `sft.jsonl`. But `run_start.task` is the full string
the run was told, preamble included (contract 5.1), and it is the run's first user turn: every
`hydra run` provider receives it as the prompt. So no exported SFT line has the turn the
trajectory answers.

Measured on 2026-10-04 by the session: `hydra export` over this machine wrote 351 lines, 226 with
tool calls and 3,722 calls between them, and 99 carry any user message at all. The ones that
exist are injected skill bodies or interactive turns, not the task. A builder that keeps each
trajectory up to its first tool outside Read, Write, Edit, Bash and Grep
(`.scratch/hydra/training/round1/build_sft.py`) gets 26 usable trajectories, 204 calls.

1. **Does the task cross?** (A) yes, as the first `user` message, through the same redaction pass
   and secret scan as every other value, and its paths counted in eligibility like a tool call's;
   (B) yes, but with Hydra's preamble cut off the front, since the preamble is Hydra's text and
   identical on every line; (C) no, 16.1 stands, and training builds its prompts some other way
   on this machine.
2. **Does losslessness (16, "every slot filled from the record or left empty") already require
   it?** If yes, this is a gap in the build, not a decision, and the answer goes to a build ticket.

## Answer

Decided 2026-10-04 by a four-seat panel, no tie-break needed. Answers are in
`~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-04/`.

**D2: it is a decision, four to zero.** Contract 16 lets a recorded field stay behind on purpose,
and this change alters what leaves the machine, so the mapping could not simply be corrected as a bug.

**D1: the whole task crosses, three to one (A).** `run_start.task` becomes the first `user`
message of each `sft.jsonl` line, preamble included. It passes through the same redaction pass
and verification scan as every other value. Any path in it counts toward eligibility as if a tool
had touched it, so a task that names a path outside the allowlist keeps its run out of the
corpus. Codex (adversary, p 0.66), Gemini (security, p 0.85) and Fable (architect, p 0.58) picked
A. Grok (reviewer, p 0.58) picked C, since free text can paste a file that no path check sees.
Its reversal condition was an audit of the task strings.

**That audit, by the session, the same day, over the 351 eligible runs with the 632-character
preamble removed.** 18 tasks name a path outside `~/hydra`, all in `~/private-repo`, and
under A those 18 runs leave the corpus. 53 name private-repo, which the private-name class has to
redact. 56 carry fenced code; nobody has yet checked whether any of it is outside the
allowlisted repo. None carries a secret prefix. The fenced-code count is the residue Grok named,
and the build ticket checks it before the first export with tasks leaves the machine.

**Fable's note, not adopted, kept for the build.** A fourth option the brief lacked: the preamble
as a `system` message and the written task as the `user` message, which keeps the whole string and
lets a trainer drop the preamble downstream. The build ticket may take it if splitting the string
is exact; the answer above only fixes what crosses, not how many messages it fills.

Contract 16.1's `run_start` row changes with the build ticket that carries this.
