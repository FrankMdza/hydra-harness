# 98 - A gated session that launches what the human approves, seat by seat

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-02, by the session, from the human's words during the usability probes: one
Hydra, a copilot like Claude Code, that can spawn and be watched at the same time. Reopens what
map tickets 64 and 82 settled, on new evidence.

## What the session measured

- Today the two halves are separate. An ungated copilot (`hydra cockpit --session claude`)
  launches anything by shell and is itself bound by nothing. A gated one (`--policy standard`)
  is bound and launches only an approved shape through `panel`; `config/panels.json` holds one,
  `review`. `delegate` is refused inside a macOS run (`hydra/seatbelt-delegation`, map 64).
- Run `01a0fdeb` (gated): four refusals of `claude`'s own `Agent` tool, then the session did the
  work alone. Run `01a0fe17` (gated): three `delegate` refusals, nothing launched. No gated
  session reached `waiting approval`; the session writing this had told the human it would, from
  a wrong reading of `src/seats.rs`.
- The human's proposal for `config/panels.json` (outside the repo,
  `PROPUESTA-config-panels.json`) adds `architect`, `quant-review` and `research`. It is
  protected config #6 and his to write.
- Under `standard` a shell write outside the worktree succeeds: run `01a0fe11-9cb2`
  (`~/hydra-oficina/runs/`) ran `printf 'hola' > ~/prueba-fuera.txt`, the gate answered
  `hydra/allow: policy standard allows shell`, and the file exists. That is the limit
  `SECURITY.md` states and map 67 keeps (`allow default`, denies for Hydra's own paths). A tester
  read it as a hole. It is written here because a gated copilot the human hands to other people
  inherits it; it is not a second probe of the launchd class and does not reopen map 67 alone.

## Questions for the human

1. Is more shapes enough? With `research` approved a gated session sends two agents to work
   without asking. Or does the session need to ask for seats no shape names?
2. If it asks for loose seats: what does a request carry (provider, model, policy, task), and
   what does the human see before `a` in the cockpit?
3. Does a request no shape covers wait for ever, time out, or refuse after a count? Map 82's
   deny rule (three repeats in ten denials) was written for shapes.
4. Is the cap per hour per session, per human, or per machine? This one has 8 GB.
5. Does the gated copilot get a narrower `standard` than an agent does, since a person other than
   the human may be typing into it?

## Answer

Resolved 2026-10-03 by a panel of three under the human's delegation: `codex` (adversary,
`standard`), `cursor-agent` grok-4.7-high (reviewer, `judge`), `agy` gemini-3.1-pro-high
(security, `judge`). One brief for maps 96 to 98, ten decisions, none split three ways, so no
judge sat. Brief and answers in the human's decision log,
`~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/`.

- **A gated copilot that launches work (D8): more shapes only**, 2 to 1 (grok 0.67, agy 0.65;
  codex took shapes plus loose requests held for a key, 0.77). Shapes are configuration the
  human writes, so `research` and `architect` add no new launch path; a loose request that names
  its own policy could start a wider agent on one key press. The human writes the shapes into
  `config/panels.json` (protected config #6); his proposal already holds them.
- **A request no shape covers (D9): refused as denied after a timeout**, unanimous (0.64, 0.56,
  0.60). This is already the code: since build 141 such a request holds the call open for
  `APPROVAL_WAIT`, 600 seconds (`src/seats.rs:89`), and answers as refused after. Nothing to
  build.
- **When someone other than the builder types (D10): a narrower sandbox for the gated copilot**
  that also denies writes outside the worktree and the temp directory, unanimous (0.91, 0.81,
  0.95). Map 67 is not reopened for agents: this narrows the gated interactive session only.

Reversed by: more than 3 of the next 5 people needing a launch that is not `review`, `research`
or `architect` (D8); more than 3 in 10 timed-out requests being ones the human would have
approved (D9); the narrower sandbox refusing ordinary work in more than 2 of 5 gated copilot
sessions, or the provider needing to write its own state outside the worktree (D10).

Carried by build ticket 250 (the narrower sandbox) and by the human's shapes.
