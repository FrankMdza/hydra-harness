# 01 - Provider headless modes and structured output

Type: research
Status: resolved

## Question

For each of the four providers, what exactly can Hydra drive from a subprocess, and what structured events come back?

Cover per provider: the headless invocation, the machine-readable output format and its event vocabulary, whether reasoning or thinking tokens are exposed at all, how tool calls and their results appear, how permission prompts are surfaced and answered, session ids and resume, and how a run terminates.

- `claude` 2.1.252
- `cursor-agent` 2025.10.02, including its ACP mode
- `gemini` 0.42.0
- `hermes` 0.14.0, its `-z` flag and its `acp` mode

The record schema cannot be designed before this is known: the schema can only carry what the providers actually emit. Name the fields that are missing from one or more providers, because those become the lossy edges of the contract.

Primary sources only: `--help` output on this machine, and each vendor's own docs. The originating research doc is not a source.

## Answer

The survey is complete for all four providers plus Antigravity. Read the findings file; three things matter more than the rest.

**Drive ACP, not print mode.** In default print modes only five fields survive all four providers, and two of those only because Hermes' can be reconstructed after the run. If Hydra drives `gemini --acp` and `hermes acp` instead, the common set widens to streamed text, thought chunks, tool calls with args and results, and answerable permissions. Cursor becomes the only laggard, and stays one, because it has no ACP.

**Gemini's OAuth path is dead on this machine.** `gemini` 0.42.0 with `oauth-personal` returns `IneligibleTierError: UNSUPPORTED_CLIENT`, because Google retired the individual Code Assist tier for this client and points at Antigravity instead. So the Google side today needs a `GEMINI_API_KEY` or a Vertex project, which moves it onto the raw-model plane that D4 reserves for API keys. That is a new decision, now ticket 18.

**Ten named gaps** become the lossy edges of the record contract, listed in full in the findings. The load-bearing ones: reasoning is Claude-only in print mode and never available from Cursor at all; Cursor emits no token counts, so no per-run token figure for Cursor is anything but a guess, which hits D12 directly; permission decisions are answerable from a parent on Claude alone; and termination is not guaranteed to arrive as a frame, so process exit is the authoritative end of a run and the `result` frame is optional enrichment.

Three things Hydra must own rather than read back: its own run id (Hermes cannot be told one and does not print the one it used), its own per-line timestamps (only Gemini stamps every frame), and delegation lineage recorded by the parent at spawn (only Claude and Hermes carry native parent links, and Cursor has none).

Detail: [research/01-provider-headless-survey.md](../research/01-provider-headless-survey.md)
