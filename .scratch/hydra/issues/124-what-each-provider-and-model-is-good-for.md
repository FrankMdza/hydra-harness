# 124 - What each provider and model on this machine is good for, from what the records say

Type: research
Status: claimed
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
