# 124 - What each provider and model on this machine is good for, from what the records say

Type: research
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from the human's note of the same day ("una investigación
sobre los mejores usos de cada proveedor que tenemos disponible en nuestro harness"). Research:
the findings are read from the records and the tickets, not asked of the models.

## Question

Four CLIs and, on 2026-10-15, these models: `claude` with Opus 5.5, Sonnet 5.5, Haiku 4.5 and
Fable 5.1; `cursor-agent` with Grok 4.7 (low to xhigh), Grok 4.6, Grok 4.5, Composer 2.5, GPT
5.6 Sol and Luna, Codex 5.3, Gemini 3.7 Flash, Opus 5, Sonnet 5 and Fable 5 through Cursor's
quota; `agy` with Gemini 3.8 Flash (three efforts), Opus 5.5, Sonnet 5.5 and GPT-OSS 120B;
`codex` with GPT 5.6 Sol and GPT 6 Astra. Over a month of recorded runs, build Outcomes, panel
memos and the memory the sessions kept, which of them did what well, what badly, at what cost,
and what should each be handed by default?

1. Per provider and model: the tickets it built that went to master as they came, the ones that
   needed a mechanism correction, the ones that failed; tokens and wall time per ticket; the
   panel seats it sat and whether the synthesis trusted its claims.
2. The failure shapes that repeat: agy's own timeout on large context, codex's quota window,
   Cursor's premium allowance, Composer calling unhooked tools, Haiku's degenerate logic,
   Sonnet's token spend on cache reads.
3. A recommended default per kind of work (Rust build with a suite, small script, read-only
   measurement, research with sources, panel seat, judge), with the evidence line that backs each
   and the open question where there is none.

## Findings

Resolved 2026-10-15. The long version, every claim with its run id, ticket or note, is
[research/124](../research/124-provider-best-uses.md). Read from 932 records, 259 build Outcomes
that resolve to a record, six syntheses, four panel reviews and the memory notes.

| Work | Default | Fallback | Evidence |
|---|---|---|---|
| Rust build with a suite | claude Opus 5.5 | codex GPT 6.1 Sol, then Grok 4.6 High | Opus 41 of 70 as is, 3.8M, 7 min; Sol 12 of 27, 1.2M, 6 min; Grok 4.6 12 of 17, 4.7M, 10 min |
| Small script | codex GPT 6.1 Sol | claude Sonnet 5.5 | Sonnet 5.5 8 of 12 as is at 0.7M; no head-to-head |
| Read-only measurement | agy Gemini 3.8 Flash, small context | claude Haiku 4.5 | `01a08008-cb50` 49K tokens; Flash dies on large context |
| Research with sources | claude Sonnet 5.5, capped | claude Opus 5.5 | remembered only; map 116 open |
| Panel seat | Opus 5.5, Astra, Grok 4.7 High, Flash declared | Fable 5.1 | Opus and codex wrong on no checked finding; Flash wrong on 4 of 6 seats of 13 Oct |
| Closed-shape judge | codex GPT 6 Astra | Haiku 4.5 | Astra 2 of 2 at about 20K; Composer gated 6 of 6 |
| Split-panel judge | Fable 5.1 | Astra beside it | map 106, both chose A blind |

- Never give agy the Rust suite: `01a07efb-de49` and `01a07ffe-4808` timed out, `01a13736-9575`
  broke at 14.9M.
- Cache reads are 84 to 97 percent of tokens on every model. Sonnet 5 stands out by volume, median
  10.6M per ticket, not by share.
- Codex's window reset is unknown today; 16 codex runs ended `rate_limited` this month.
- Memory numbers drift from records: `01a0d3e2-1098` took 171 s, remembered as 14 to 25 min.
- `hydra-puppets report tickets` counts build 312's pasted table as a ticket, so every model gains
  one ticket and one `rejected`; its prose classes leave 299 of 327 tickets `unsaid`.
- No measurement of any judge against the human, of Grok 4.7's claims one by one, or of 12 listed
  models; GPT-OSS is map 125's.
