# 127 - May a shell under `standard` read the run's own pinned skill snapshot

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from map 120's answer, as the question 6 the two seats never
got to vote on. Grilling: the human answers, the session only asks and brings what it measured.

## Question

A run's linked skills under `.claude/skills/` resolve to `~/hydra/skills-snapshots/<sha>/...`,
and the pattern floor refuses any shell command that names `~/hydra`'s runtime directories. So a
shell `cat` of a linked skill is refused on every run that has skills linked, on three of four
providers (`codex` gets a copy since map 71). The spec's sentence "a shell command naming the root
is how `cursor-agent` and `agy` read a skill" is true of `ls` on the root and false of `cat` on a
file in it. Opus measured the refusal twice in its own run on map 120, and found that the two
tests that would catch it (`tests/gate.rs` lines 2543 and 2601) never link the skill, so they pass
for the wrong reason (build 320 fixes the tests, not the rule).

1. Should the token pass's own list include the run's own `skill_snapshot` directory, compiled
   only when `run_start.sandbox` is set, since the profile denies writes there and the kernel
   already allows the read? That is Opus's proposal and it changes what `standard` lets a shell
   read.
2. If yes, is the snapshot the only read-only runtime directory a run should reach by shell, or
   does the same reasoning apply to anything the seatbelt allows reading? The design's answer
   so far has been that the floor is a deny list of strings and the seatbelt the real boundary,
   so widening the floor is a documentation question as much as a security one.
3. If no, how does a `cursor-agent` or `agy` run read a skill file at all today, and should the
   preamble (protected) say "use your file tool, not the shell" for skills?

## Answer

Resolved 2026-10-15 by the human, who took the session's recommendation from the panel 130 synthesis as it stands ("me gustan tus recomendaciones", "sigue tus recomendaciones para el resto").

1. Not now. The file tools already read a linked skill on `claude`, `cursor-agent` and `agy`
   (`tests/gate.rs` asserts the allow under `read-only`, `standard` and `curate`), so what the
   floor costs is a shell `cat`, and what an exemption costs is the floor's one property, that it
   is a deny list of strings the token pass cannot tell `cat` from `cp` on. The panel split two to
   two (Opus and Gemini for the exemption when wrapped, Grok against, GPT only once a field says
   which wrapper is in force). Revisit when 316's `sandbox_kind` exists, so an exemption can be
   conditioned on `seatbelt` and not on a non-null sha, and then only with five tests: allowed
   when wrapped, refused when not, a sibling sha refused, `spawn/` refused, and a write through the
   link failing at the kernel.
2. If it ever comes, the own snapshot only. The run's spawn directory holds the gate's source and
   `launch.json`; it stays out.
3. The preamble stays as it is. The sentence "a shell command naming the root is how `cursor-agent`
   and `agy` read a skill" in the harness spec is true of `ls` and false of `cat`, and the spec gets
   amended with 316's follow-up, not the preamble.

Tickets: none opened; the revisit is written here. Panel memos and synthesis under `research/130-*`.
