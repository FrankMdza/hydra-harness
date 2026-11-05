# 124, what each provider and model is good for, from the records

Written 2026-10-15 for map ticket 124. Every line says where it comes from. "Measured" means a run
record, a ticket Outcome, a review that checked findings against code, or a synthesis that checked
seats against the repo. "Remembered" means a note under the session memory directory, which is what
a session wrote down and may not match a record.

## What was read

- The 18 memory notes under `~/.claude/projects/-Users-USER-hydra/memory/`, most of all
  `hydra-provider-limits.md`, `proveedores-paneles-y-tareas.md`, `hydra-orchestration-mode.md`,
  `orquestador-solo-opus-o-fable.md` and `hydra-loop-autonomo.md`. Remembered.
- 932 run records under `~/hydra/runs/`, each read through `hydra show` for outcome and usage, with
  wall time taken from the first and last `ts` of the file. Measured. Token figures below are
  `hydra show`'s `total_tokens`, which counts cache reads; the record's own provider line often
  reports a smaller number without them.
- 327 build tickets under `.scratch/hydra/build/`. 259 of them name at least one run id in
  `## Outcome` that resolves to exactly one record. Measured.
- `hydra-puppets report tickets` run today, 55 s. Measured, with a caveat in the failure shapes
  section.
- Six syntheses and 18 seat memos: `research/117` to `123`, and the three `88-seat-*` memos with map
  88's Answer. Measured where the session checked a claim.
- The panel reviews under `docs/reviews/`: the seven readers of 2026-09-22, the second, third and
  fourth panels, and the first Master of Puppets goal. Measured where the review checked a finding.
- The two goal logs under `~/hydra/puppets/`. `sheet-cache.json` is a 214 MB per-run cache from
  2026-09-25 and holds nothing the records do not.
- Not read: the panel answers of maps 100 to 107 under `~/.claude/handoffs/`, outside this repo.
  Their tickets' Answers give the votes but not a check of each seat's claims.

### How the per-ticket numbers were built

For each ticket the builder is the completed run with the most tokens among the runs its Outcome
names. That rule is wrong where a review outspent the build, as in ticket 113, whose 15.7M Opus run
was the fourth panel's reader and whose builder was Composer. The class of each ticket comes from a
regex over the Outcome prose: "applied as written", "unchanged", "reviewed and applied" count as
applied as is; "the session changed", "corrections by the session", "amended" count as corrected;
"usage limit", "at capacity", "the session finished" count as finished by hand; "not applied" or
"rejected by the session" count as rejected. Anything else is unsaid. The ticket lists are given so
a reader can disagree with any one of them.

## Per provider and model

### claude, Opus 5.5

Built 70 tickets by the rule above, from build 91 to 286, Hydra and the puppets port. Measured.

| Class | Tickets | Numbers |
|---|---|---|
| applied as is | 41 | 140 141 147 150 151 166 167 171 to 174 176 to 179 183 184 186 187 191 194 195 199 210 213 215 218 224 231 to 234 242 243 247 250 264 266 267 271 272 |
| corrected | 9 | 91 96 139 142 143 169 196 212 290 |
| finished by hand | 3 | 137 207 255 |
| unsaid | 16 | 113 156 170 180 182 192 193 214 220 221 251 257 259 275 279 286 |

- Tokens per ticket: median 3.8M, largest 37.7M on ticket 286's second run `01a10504-aeab`, 112 min.
  Wall time median 7.2 min. Measured.
- Of 125 runs named `claude-opus-5-5`, 122 completed and 3 have no outcome yet, two of them live
  today. Of 72 under the alias `opus`, 66 completed, 5 were `gated` and 1 `rate_limited`. None
  `failed`. Measured.
- Build 207's run stopped at the account's usage limit and the session applied its unfinished diff.
  Measured, build 207 Outcome.
- Build 199, run `01a0f5ea`, 19.9M tokens, 45 min, "applied to master as written"; build 194, run
  `01a0f616`, 7.6M, "applied to master as written". Measured, those Outcomes.
- The session remembers that every Opus review in the 23 Sep batch found at least one mechanism
  fault, and that in build 92 the agent's test passed with the wiring removed. Remembered,
  `hydra-provider-limits.md`.
- Reading a whole repository through `hydra run` is expensive on the Claude window: `01a0c793-4b3c`
  spent 9.26M and ended `rate_limited`, and the fourth panel reader `01a0d3e4-4245` spent 15.7M, 97
  percent cache reads. Measured. The session remembers the first moved `five_hour` from 0.14 to 0.71.
  Remembered.

Panel seats. Opus is the seat the reviews and syntheses trust most.

- Third panel: 24 findings, none wrong. Fourth panel: 20, none wrong. Second panel: all checked out
  except G27, partly right, and G26, where codex was right. Measured, the three reviews.
- Map 118: Opus said the plan-to-outcome join exists; the GPT seat read `labels.rs` and found it
  does not. Map 120: Opus's guess at what refused `01a1332c` was wrong, as the session found on the
  record, while its newline measurement beat Gemini's reading. Measured, those syntheses.
- Maps 119, 122 and 123: Opus found the finding the other seats missed each time: the Darwin-only
  `procinfo.rs`, the live panel that cannot compose, reap exiting 128 on a gitlink. Measured.
- Seat cost on 2026-10-13: 1.55M to 4.13M tokens, 2.9 to 7.0 min, six seats. Measured, runs
  `01a133fc-9d76`, `01a13427-b37a`, `01a13719-3904`, `01a13730-dc57`, `01a13747-7055`,
  `01a13753-08be`.

### claude, Sonnet 5 and Sonnet 5.5

| Model | Tickets | As is | Corrected | Unsaid | Median tokens | Median min |
|---|---|---|---|---|---|---|
| Sonnet 5, 7 to 24 Sep | 9 | 1: 18 | 2: 95 136 | 6 | 10.6M | 16.8 |
| Sonnet 5.5, from 29 Sep | 12 | 8: 144 149 152 154 155 160 161 165 | 1: 146 | 3 | 0.7M | 2.7 |

- Ticket 27's run `01a07f37-9cab` spent 54.7M tokens in 41 min. `01a0d237-39db` on private-repo
  spent 24.2M and ended `rate_limited`. Measured.
- The session remembers three Sonnet runs in a row losing their last third re-verifying corpus-walk
  failures master had already fixed, and a research run with WebSearch that spent 11.0M tokens and
  moved `seven_day` from 0.18 to 0.42. Remembered, `hydra-provider-limits.md`.
- The session remembers that Sonnet opened a linked skill on its first call and Haiku never did.
  Remembered, `hydra-provider-limits.md`, 2026-09-09.
- Sonnet never orchestrates or reviews, by Frank's rule of 2026-10-03. Remembered,
  `orquestador-solo-opus-o-fable.md`.
- No panel seat on record.

### claude, Haiku 4.5

- 81 runs, 72 completed, 5 `rate_limited`, 3 `gated`. Median 0.24M tokens. Most are probes and
  judges. Measured.
- Two Rust build tickets: 25, 4.7M, and 82, 4.0M. Neither Outcome says corrected. Measured.
- As rung two's proposer in build 36's fourth pass, `01a0822e-bc48`, Haiku spent 8.07M tokens and
  invented a lowercase `bash` tool that `claude` refused. Measured, build 36 Outcome.
- On a private repository its code had defects and its documentation held six false claims. Remembered,
  `hydra-provider-limits.md`, 12 and 14 Sep. No record of that review is in this repo.
- As a judge under `judge`: 4 of 5 completed, 20K to 31K tokens. Measured.

### claude, Fable 5.1

- 9 runs, 8 completed. Three as a closed-shape judge at 18K to 20K tokens, three under `permissive`
  at 0.8M to 1.9M. Measured.
- Map 106: the panel split 2 to 2 and Fable, run `01a104e7-e73d`, and codex Astra, `01a104e7-e730`,
  both chose A, blind and reading in opposite orders. Measured, map 106 Answer.
- The session remembers that a Fable seat on 2026-09-26 turned the epoch-cut panel from D to E.
  Remembered, `proveedores-paneles-y-tareas.md`.
- One judge run, `01a11311-c651`, ended `gated`. Measured.
- No build ticket.

### cursor-agent, Composer 2.5

Built 76 tickets, the most of any model, from 20 to 263. Measured.

| Class | Tickets | Numbers |
|---|---|---|
| applied as is | 19 | 31 61 62 63 64 66 67 69 110 118 121 126 127 135 148 202 211 217 222 |
| corrected | 21 | 65 68 79 93 108 117 119 120 122 123 125 133 134 197 198 206 241 246 253 258 263 |
| finished by hand | 5 | 92 111 208 209 239 |
| rejected | 1 | 240 |
| unsaid | 30 | the rest |

- Tokens per ticket: median 1.5M, largest 17.3M on ticket 217. Wall time median 3.6 min. Measured.
- Build 197: "Five corrections by the session", each a way to lose a good line. Build 108: "the
  session's review changed five mechanisms before master". Build 240: not applied, because the run
  ran `git stash` and applied a stash entry from September into its worktree. Measured, those
  Outcomes.
- The first Master of Puppets goal, `ea69da1f`: five Composer tasks, three verdicts `applied`, two
  `fixed`, one of them a benchmark it never measured. Measured, goal log and
  `docs/reviews/2026-09-24-master-of-puppets-first-goal.md`.
- 43 of 183 Composer records hold frames of Cursor tools that run without the hook:
  `readLintsToolCall` 50, `awaitToolCall` 38, `webSearchToolCall` 60, `webFetchToolCall` 54,
  `updateTodosToolCall` 10, `getMcpToolsToolCall` 7. These are frame counts, not call counts.
  Measured, raw records.
- Every Composer report in a worktree called three `run::tests` failures pre-existing, and they
  came from the worktree's `git init`. Remembered, `hydra-provider-limits.md`, 24 Sep. Build 114's
  Outcome names the same three. Measured.
- Under `judge`, 6 of 6 Composer runs ended `gated`. Measured.
- Frank retired it as a driver on 2026-10-03. Remembered, `proveedores-paneles-y-tareas.md`.

Panel seats. Second panel: three smaller findings wrong and half of a fourth. Third panel: 3 of 15
wrong. Fourth panel: 1 of 12 wrong, and nothing new beyond Opus. Measured, the three reviews.

### cursor-agent, Grok 4.6 High and Grok 4.6 High Fast

| Model | Tickets | As is | Corrected | Unsaid | Median tokens | Median min |
|---|---|---|---|---|---|---|
| Grok 4.6 High, Oct | 17 | 12: 235 237 260 265 269 274 276 280 283 285 287 288 | 3: 236 273 289 | 2: 291 292 | 4.7M | 9.8 |
| Grok 4.6 High Fast, Sep | 4 | 1: 35 | 0 | 3: 21 30 38 | 2.4M | 10.6 |

- Two Grok 4.6 runs on 2026-10-06, `01a11344-9c2e` and `01a1134c-0a5d`, died on Cursor's
  `RetriableError: Connection failed repeatedly`. Measured.
- `01a0a56e-c71f` died on signal 9 with six runs open, an out-of-memory kill on this 8 GB machine.
  Measured; the cause is remembered, `hydra-provider-limits.md`.
- Build 30 needed a mechanism correction before master: its staging failed on the first real reap.
  Remembered, `hydra-orchestration-mode.md` and `hydra-provider-limits.md`.
- No panel seat on record.

### cursor-agent, Grok 4.7 High

- 7 build tickets, 158 to 190: 6 applied as is, 162 corrected. Median 3.3M tokens, 17.3 min.
  Measured.
- 43 runs, 41 completed. Under `judge`, 15 of 16 completed at 27K to 112K tokens, nearly all of them
  inline decision-panel seats from 29 Sep to 6 Oct. Measured.
- Seat on map 88 as researcher, `01a0f0ca-7ed1`, 1.71M tokens. The Answer records no false claim
  from any of the three seats. Measured, map 88 Answer.
- Seat on maps 100, 104, 106 and 107. It was in the minority once, D2 of map 104. Measured, those
  Answers. No synthesis checked its claims one by one.
- Read-only reviews of the port builds: 3M to 6M tokens, real findings in nearly every build.
  Remembered, `hydra-provider-limits.md`, 1 Oct.
- `01a13400-1a79` on 2026-10-13 died at its first turn on Cursor's free plan. Measured.

### cursor-agent, premium models through Cursor

`claude-opus-5-thinking-high` 5 runs and GPT 5.6 Sol through Cursor 7 runs: 10 `rate_limited`,
1 failed, 1 completed. Measured. Cursor's premium allowance ran out on 2026-09-07 and reset on
2026-09-29. Remembered. There is no measurement of a finished build or seat on any of them.

### codex, GPT 6.1 Sol

Built 27 tickets, 159 to 301. Measured.

| Class | Tickets | Numbers |
|---|---|---|
| applied as is | 12 | 200 201 203 223 244 262 270 277 278 282 284 299 |
| corrected | 6 | 238 252 254 256 268 293 |
| unsaid | 9 | 204 216 219 248 295 296 297 298 301 |

- Ticket 281's first attempt, `01a10553-f215`, was rejected by the session: it split a file with
  `include!`, one module in seven files. Ticket 300's run `01a13431-b2d5` died at its last turn on
  "Selected model is at capacity" and the session finished it. Measured, those Outcomes.
- Tokens per ticket: median 1.2M, largest 5.7M. Wall time median 5.6 min. Measured.
- 59 runs, 54 completed, 3 `rate_limited`. Measured.
- Map 88's adversary seat, `01a0f0d5-51ae`, 0.23M tokens; its point that a child keeps Hydra free of
  any model or credential became the decided shape. Measured, map 88 Answer.
- The session remembers four cockpit builds that went in without correction. Remembered,
  `hydra-provider-limits.md`, 1 Oct.

### codex, GPT 6 Astra and the unnamed codex runs

- Astra: 28 runs, 24 completed, 4 `rate_limited`. One build ticket, 313, 4.2M. Measured.
- Unnamed codex runs before the model was recorded: 86 runs, 67 completed, 9 `rate_limited`, 7
  failed. The session remembers that the hook payload named `gpt-6-astra` then. Remembered.
- Second panel: none of 18 findings wrong, and on G26 it corrected Opus. Third panel: none of 19
  wrong, eleven new. Measured, the two reviews. The session remembers it as "the hardest and most
  useful review", run `01a0c793-726c`. Remembered; the record gives 3.84M tokens, not the 7.4M the
  memory and the review quote.
- Map 117 and 118: the adjudicator seat was right on every claim the seats disputed, 0.33M and
  0.65M tokens. Measured, those syntheses.
- Closed-shape judge: `01a0c4c9` answered `{"verdict":"pass","findings":[]}` in about 16K tokens by
  the record, the cheapest of its panel. Measured. Under `judge` before map 63 it was gated at its
  first read because codex reads only by shell, `01a0d229-ffd9`. Measured and remembered.
- Read-only reviews of the port builds: 0.5M to 1.3M tokens with real findings. Remembered.
- Codex refused the attack-corpus task of map 68 as a cyber risk. Remembered,
  `hydra-hiring-panel-verdict.md`.
- The three Astra seats of maps 120, 122 and 123 died at their first turn on the usage limit,
  `01a13747-35ec`, `01a13752-cc38`, `01a13761-4b98`. Measured.

### codex, GPT 5.6 Sol

10 runs, all completed, all on private-repo and HephaestusLab, median 0.62M. Measured. No Hydra
build ticket and no seat.

### agy, Gemini 3.1 Pro

- 5 build tickets: 157 as is, 94 corrected, 19 and 33 failed on the same run. Median 1.1M. Measured.
- `01a07efb-de49` died on "timeout waiting for response". Measured.
- Under `judge`, 14 of 15 completed at 17K to 34K tokens as an inline decision seat. Measured.
- First panel: read eight files and gave the highest scores. Second panel: cites documents only and
  3 of 7 weaknesses wrong. Third panel: 1 of 8 wrong, "for the second panel in a row". Fourth
  panel: skipped `src/` and `tests/`; nine items restated findings already closed. Measured, the
  four reviews.
- The Master of Puppets design panel: eight citations, two false, and a proposal that `hydra resume`
  rejects. Remembered, `hydra-provider-limits.md`, 24 Sep.
- Gone from agy 1.3.4: `01a13401-5ea3` died on "invalid model selection" on 2026-10-13. Measured.

### agy, Gemini 3.8 Flash

- High: 19 runs, 15 completed, 3 failed. Low: 7 runs, 7 completed, median 0.03M. Measured.
- Short closed probe: `01a08008-cb50` answered in 49K tokens and 0.2 min. Measured.
- Large context: `01a07ffe-4808` died on "timeout waiting for response" after reading 3.2M tokens.
  Measured. `01a0919b-fa7d` read the repo for 92 steps and hit its own five-minute print timeout
  without writing. Remembered; the record gives 3.14M, the memory 264K.
- Build 115, four comments, cost 3.7M tokens. Measured. The session decided never to give it even
  comments again. Remembered.
- Panel seats on 2026-10-13:

| Map | Run | Tokens | Min | Outcome | What the synthesis found |
|---|---|---|---|---|---|
| 117 | `01a1342b-bcaa` | 0.74M | 6.5 | completed | its claim that 117 needs 115 first was false |
| 118 | `01a13402-282c` | 0.88M | 35.3 | completed | its rows rule would drop plans the contract allows; its 100 is a proposal |
| 119 | `01a1371f-d264` | 16.52M | 14.0 | completed | assumed a Linux binary exists, which is false; its `realpath` claim was true |
| 120 | `01a13736-9575` | 14.94M | 17.8 | failed, "stream was interrupted" | its newline reading was wrong against Opus's measurement; its message wording would have been false |
| 122 | `01a1374a-6210` | 7.86M | 8.8 | completed | described the compose mechanism without seeing it cannot fire |
| 123 | `01a13756-d51d` | 6.03M | 11.1 | completed | no false claim; did not look at reap |

### agy, GPT-OSS 120B

One run, `01a13dff-47d9` today, ended exit 3 on "UNAVAILABLE (code 503): No capacity available for
model gpt-oss-120b-medium". Measured. Map 125 measures it against Flash and is still claimed. No
other measurement.

### Models with no record

Nothing on record for: Grok 4.7 low,
medium or xhigh, Grok 4.5, GPT 5.6 Luna, GPT 5.3 Codex, Gemini 3.7 Flash through Cursor, Sonnet 5 or
Fable 5 through Cursor, agy's Opus 5.5 and Sonnet 5.5, `gemini-3.8-flash-medium`. No measurement.

## The failure shapes that repeat

| Shape | Runs or tickets | Source |
|---|---|---|
| agy times out on its own model wait when the context is large, whatever the model | `01a07efb-de49`, `01a07ffe-4808`; `01a13736-9575` broke at 14.9M on "stream was interrupted" | measured |
| codex runs out of quota, and the binding window is the weekly one | 16 `rate_limited` codex runs on 23 and 24 Sep, 1 Oct and 13 Oct; `01a13431-b2d5` at capacity | measured; the weekly window is remembered |
| Cursor's premium allowance and plan | 10 `rate_limited` premium runs, 7 Sep to 29 Sep; `01a13400-1a79` on the free plan | measured; reset dates remembered |
| Cursor connection drops | `01a11344-9c2e`, `01a1134c-0a5d` on 6 Oct | measured |
| Cursor tools that run without the hook | 43 of 183 Composer records; 6 of 43 Grok 4.7; 5 of 23 Grok 4.6; 4 of 10 Grok 4.6 Fast | measured |
| Composer gated under `judge` | 6 of 6 | measured |
| Composer and Haiku invent or mislabel | Haiku's lowercase `bash`, build 36; Composer's unmeasured benchmark, goal `ea69da1f`; "pre-existing" failures that were the worktree's, build 114 | measured |
| Haiku's defects on a private repository | 12 and 14 Sep | remembered only |
| Token spend is mostly cache reads, on every model | cache share 0.97 Opus 5.5, 0.97 Sonnet 5, 0.95 Composer, 0.95 GPT 6.1 Sol, 0.93 Flash High, 0.84 Gemini Pro | measured |
| A reader that does not open the Rust is wrong more often | wrong on 6 of 21 in round two and 4 of 23 in round three, against none for the two who opened it | measured, third panel review |
| Out-of-memory with six runs open | `01a0a56e-c71f` | measured |

The ticket names Sonnet's cache reads as a failure of its own. The share is the same on every model.
What sets Sonnet 5 apart is volume per ticket: median 10.6M against 3.8M for Opus 5.5 and 1.2M for
GPT 6.1 Sol.

The memory and the records disagree on numbers more than once. Composer's builds 113, 116 and 117
are remembered at 14 to 25 min each; `01a0d3e2-1098` lasted 171 s by its own `duration_ms`. Codex
review `01a0c793-726c` is remembered at 7.4M and `hydra show` gives 3.84M. Read a memory's numbers as
a lead and take the figure from the record.

`hydra-puppets report tickets` counts build 312's Outcome as a ticket. That Outcome pastes the
whole previous report, so every model in the per-model table gains one ticket and one `rejected`.
Its prose classes leave 299 of 327 tickets `unsaid`. The per-ticket table is usable; the prose
columns of the per-model table are not yet.

## A recommended default per kind of work

Cursor is on Pro again as of today. When codex's quota window resets is unknown, so every codex
default below has a fallback.

| Kind of work | Default | Fallback | Evidence | Open question |
|---|---|---|---|---|
| Rust build with a suite | claude Opus 5.5 | codex GPT 6.1 Sol, then Grok 4.6 High | Opus: 41 of 70 as is, 9 corrected, median 3.8M, 7.2 min. Sol: 12 of 27 as is, 6 corrected, median 1.2M, 5.6 min. Grok 4.6: 12 of 17 as is, median 4.7M, 9.8 min | The Claude window is shared with the orchestrating session; no record ties a build to its share of `seven_day` |
| Small script | codex GPT 6.1 Sol | claude Sonnet 5.5 | Sol's cheapest builds run 0.2M to 0.9M. Sonnet 5.5: 8 of 12 as is, median 0.7M, 2.7 min, on small Hydra tickets | Composer was faster still at 3.6 min but corrected on 21 of 76 and is retired; no measured head-to-head on one script |
| Read-only measurement | agy Gemini 3.8 Flash, low or high, for one closed question on small context | claude Haiku 4.5 | `01a08008-cb50` 49K tokens; Flash Low 7 of 7 completed at median 0.03M; Haiku median 0.24M over 81 runs | Flash dies past a few million tokens of context; map 125 is measuring GPT-OSS on the same probes |
| Research with sources | claude Sonnet 5.5, with a cap on words and sources | claude Opus 5.5 | Remembered only: Sonnet research at 1.6M, 6.0M and 11.0M tokens | No measured comparison of research quality on any model; map 116, the research seat, is open |
| Panel seat | Opus 5.5 always, codex GPT 6 Astra, Grok 4.7 High, agy Flash declared as Flash | Fable 5.1 in place of a missing seat | Opus wrong on none of 44 findings in the third and fourth panels, codex on none of 37 in the second and third; Astra right on every disputed claim of 117 and 118; Flash wrong on 4 of 6 seats of 13 Oct, blind on a fifth, and three to four times Opus's tokens on 119 to 123 | Grok 4.7 has never had its claims checked one by one by a synthesis |
| Judge, closed shape | codex GPT 6 Astra | claude Haiku 4.5, then Gemini or Grok under `judge` | Astra 2 of 2 under `judge` at 18K to 24K; Haiku 4 of 5 at 20K to 31K; Composer 6 of 6 gated | No measurement of any judge's verdicts against the human's |
| Judge of a split panel | claude Fable 5.1 | codex GPT 6 Astra beside it | Map 106: Fable and Astra both chose A blind; Frank's rule names Fable | Two agreeing tie-breaks are one data point |
| Orchestration and review | claude Opus 5.5 or Fable 5.1 | none | Frank's rule of 2026-10-03, remembered | Map 118 asks whether a small orchestrator could do it; map 121 found Ministral 3B ties only on the typo |

Never give agy a ticket that runs the Rust suite: two timeouts on record and one interrupted stream.
Never give Composer a `judge` seat. Never launch Opus through `hydra run` to read a whole repository
when the Claude window is past half, by `01a0c793-4b3c`.
