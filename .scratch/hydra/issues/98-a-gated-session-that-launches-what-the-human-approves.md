# 98 - A gated session that launches what the human approves, seat by seat

Type: grilling
Status: open
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
