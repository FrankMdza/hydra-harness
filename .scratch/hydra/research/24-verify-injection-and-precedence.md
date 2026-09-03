# 24 - Injection shape and skill precedence, verified against current builds

Written incrementally, 2026-09-03. Every claim below is one command whose output is quoted or
summarised in place. The salvaged evidence from the agent that died on 2026-09-02 is re-checked
rather than re-derived, and where a build moved under it that is said out loud.

## Builds under test

Checked before any probe ran.

| CLI | Version today | Version the salvaged evidence ran against |
|---|---|---|
| `claude` | 2.1.257 | 2.1.257, unchanged |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026.08.31-4057e58 or later, moved |
| `agy` | 1.1.24 | 1.1.23 at ticket 10's third pass, moved |

## The dead agent's rig survived, and its raw outputs are on disk

The rig is still at
`/private/tmp/claude-501/-Users-USER/f2c096c9-40e2-4c31-bdb1-bdcf8d1f4683/scratchpad/t24/`, intact:
a store git repo, a snapshot worktree at sha `57dd5052`, a user repo, and a linked run worktree
`wt-run1` carrying `.claude/skills/`, `.cursor/skills/`, `.agents/skills/`, `.codex/` and `.grok/`.
So the salvaged claims can be checked against the JSON the CLIs actually returned rather than
against a summary of it.

`claude-run1.json`, the one paid Claude run, returned seven names:

```
t24-clash        | Marker CLASH-USER-CLAUDEROOT
t24-user-only    | Marker USERONLY-USER
t24-alpha        | Marker ALPHA-STORE
t24-dirname-only | Marker IDENT-PROJECT   (frontmatter name is t24-frontmatter-name)
t24-fmclash-dir  | Marker FMCLASH-PROJECT
t24-repo-own     | Marker REPOOWN-PROJECT
t24-rootclash    | Marker ROOTCLASH-CLAUDEROOT
```

`cursor-run1.json` returned nine, with `t24-clash` listed twice:

```
t24-clash          | Marker CLASH-USER-CURSORROOT
t24-cursoruser-only| Marker CURSORUSER-USER
t24-user-only      | Marker USERONLY-USER
t24-alpha          | Marker ALPHA-STORE
t24-clash          | Marker CLASH-PROJECT
t24-dirname-only   | Marker IDENT-PROJECT
t24-rootclash      | Marker ROOTCLASH-CURSORROOT
t24-fmclash-dir    | Marker FMCLASH-PROJECT
t24-repo-own       | Marker REPOOWN-PROJECT
```

Two things fall out that the salvaged summary did not say.

Claude listed `t24-clash` once, with the user-scope marker, and it listed `t24-dirname-only` under
the directory name while carrying the marker `IDENT-PROJECT` from the linked snapshot. So the
project-scope link was read, and only the colliding entry was dropped. The drop is per name, not
per root.

Both providers rendered `t24-dirname-only` by its directory name even though its frontmatter says
`name: t24-frontmatter-name`. That is the identity split, visible in raw JSON rather than inferred.

## The empty-description lead, reproduced from disk

This was the dead agent's last note and it is real. It cost nothing to confirm, because Cursor
writes the composed prompt to a SQLite file per session:

```
~/.cursor/chats/<project-hash>/<session-id>/store.db
```

Two tables, `blobs(id, data)` and `meta(key, value)`. The largest blob is the user turn, JSON, and
it carries the whole injected context including the skill listing. Reading it needs `sqlite3` and
no model call.

Cursor renders each discovered skill as one element:

```xml
<agent_skill fullPath="/abs/path/to/SKILL.md">the description</agent_skill>
```

The dead agent's last probe set wrote five skills into project scope, varying only the
`description` line. Four rendered. The fifth, `t24-desc-colon`, rendered like this:

```xml
<agent_skill fullPath=".../wt-run1/.cursor/skills/t24-desc-colon/SKILL.md" />
```

Self-closing. The skill is still offered to the model, by path, with no description at all.

The only thing that separates it from `t24-desc-plain`, which rendered fine, is that its
description contains the two characters `: `. Its text reads `... word wolabel: value here word
...`. The sibling probes that embed a `"quoted phrase"` and a `/slash/path` both rendered normally,
so a colon followed by a space is the trigger, not punctuation in general.

That is a YAML rule, not a Cursor quirk. A plain scalar in YAML may not contain `: `, so the
frontmatter block stops being valid YAML and the `description` key never parses. Cursor does not
report the failure, does not skip the skill, and does not fall back to the raw line. It emits the
path with an empty body.

Why this matters to ticket 10: Ouroboros writes skill files. A description it generates that
contains a colon and a space, which is ordinary English, loses its description on Cursor while
loading fine elsewhere. The model then sees a skill it cannot judge the relevance of, at a path it
can read. Ticket 10's store needs a description validator at write time, and the rule is stricter
than "valid YAML looks fine to me": the value must survive a real YAML load.

### Two more things the same transcript settles

Cursor's element has no `name` attribute. There is no name in the listing at all, only the path and
the description. So Cursor does not key on the directory name because it chose to. It has no key.
Whatever name a model reports back is the model reading the path. That is also why Cursor "does not
dedupe": with no name field there is nothing to dedupe on.

Cursor truncates a description at 207 characters and appends `...`. Twenty-four entries in that one
listing were truncated, all at exactly 210 characters including the ellipsis. A store skill whose
description runs long is silently cut on Cursor and delivered whole to the other two.

The cut is by character, not by word. Two of the twenty-four end on a space before the ellipsis,
and `t24-desc-plain`'s source description is 350 characters of repeated `word`, cut mid-token at
`... word wo...`.

## `.codex/skills` and `.grok/skills`, answered for free

The same run-3 transcript settles what nothing had been established about. Cursor read all three of
these at project scope, inside the linked worktree:

```
.../wt-run1/.codex/skills/t24-only-codex/SKILL.md   | Marker ONLY-CODEX
.../wt-run1/.grok/skills/t24-only-grok/SKILL.md     | Marker ONLY-GROK
.../wt-run1/.agents/skills/t24-only-agents/SKILL.md | Marker ONLY-AGENTS
```

Each skill existed in exactly one root, so there is no ambiguity about which root produced the
entry. Ticket 10's second pass read these roots out of the binary's strings; this is them firing.

The practical consequence for ticket 10 is a hazard rather than an option. `cursor-agent` picks up
a project-scope `.codex/skills` and `.grok/skills` that Hydra never wrote and never inspects. If a
user's repo carries either, those skills join a Cursor run beside Hydra's links, and Hydra's
launch-time collision check does not look there. The check's scope list is short by two
directories.

## The colon hazard is worse than one provider

Rebuilt the shape from scratch under `t24b/` against today's binaries: a store git repo at sha
`ae43af01`, a detached worktree of it as the snapshot, a user repo carrying its own tracked
`.claude/skills/t24b-repo-own`, and a linked worktree `wt-run` with `.claude/skills`,
`.cursor/skills` and `.agents/skills`, each holding a `.gitignore` of `*` and eight symlinks into
the snapshot.

Two of the eight vary only in the description:

- `t24b-desc-colon`, description `T24B colon probe. label: value here. Marker COLON-STORE.`
- `t24b-desc-colon2`, description `T24B colon probe two. Ratio 3:1 no space. Marker COLON2-STORE.`

`agy 1.1.24`, free, `num_turns=0` and `total_tokens=0`:

```
t24b-alpha             .agents/skills/t24b-alpha
t24b-clash             .agents/skills/t24b-clash
t24b-desc-colon2       .agents/skills/t24b-desc-colon2
t24b-desc-plain        .agents/skills/t24b-desc-plain
t24b-frontmatter-name  .agents/skills/t24b-dirname-only
t24b-tie               .agents/skills/t24b-aaa-tie
```

`t24b-desc-colon` is not there. Not empty, not truncated. Gone, with no warning on stdout or
stderr and exit 0.

So the same one-character mistake produces two different silent failures. Cursor keeps the skill
and throws away its description. `agy` throws away the skill. `t24b-desc-colon2`, whose `3:1` has
no space after the colon, survives both, which pins the trigger to `: ` and to the YAML plain-scalar
rule rather than to colons.

This is the strongest argument in this ticket for validating descriptions at write time. Ticket 10
gives Ouroboros the pen on `description`, and a colon and a space is not an exotic character
sequence. It is how anyone writes `Use when: the user asks`.

### Everything else `agy 1.1.24` confirms, at zero cost

Identity still keys on the frontmatter `name`. The entry reads `name=t24b-frontmatter-name` with
`path=.../t24b-dirname-only/SKILL.md`, so the name and the directory disagree in the same record.

The alphabetical tie-break still holds. Two directories both declaring `name: t24b-tie` resolved to
`t24b-aaa-tie`; `t24b-zzz-tie` is absent from the listing entirely.

`agy` does not truncate. The 350-character description came back whole where Cursor cuts at 207.

`agy`'s project root really is only `.agents/skills`. No entry in the listing resolved to
`wt-run/.cursor/...` or `wt-run/.claude/...`, and the repo's own `t24b-repo-own`, sitting in
`wt-run/.claude/skills/`, never appeared.

## Part one holds on today's builds

`git status` in the linked worktree reports nothing and `git add -A` stages nothing, with three
injected roots each holding a `.gitignore` of `*` and eight symlinks into the store snapshot. The
user repo's own tracked `.claude/skills/t24b-repo-own/SKILL.md` is still present in the worktree
and still listed by `claude`, because git never ignores a tracked file. Ticket 10's Q14 stands on
`agy 1.1.24` and `claude 2.1.257`, and on `cursor-agent 2026.09.02` from the section below.

## A free names oracle for Claude Code

Worth writing down because it changes what a future ticket has to pay for.

```
cd <worktree> && claude -p "x" --model bogus-model-xyz
```

Claude Code rejects the model before it calls the API, so the run costs nothing, but it has already
written the session transcript to
`~/.claude/projects/<slug>/<uuid>.jsonl`. In that file is one line of `"type": "attachment"` whose
`attachment.type` is `skill_listing`, carrying `skillCount`, a `names` array, and a `content`
string.

The catch, and it matters: on the failed run every filesystem-discovered skill renders as a bare
`- name` with no description, while bundled skills like `dataviz` still carry theirs. So the free
oracle answers identity and precedence, which are questions about names, and cannot answer anything
about descriptions. A real run renders descriptions for all of them.

Run against the `t24b` worktree it returned `skillCount 35`, and these nine names:

```
t24b-aaa-tie  t24b-alpha  t24b-clash  t24b-desc-colon  t24b-desc-colon2
t24b-desc-plain  t24b-dirname-only  t24b-repo-own  t24b-zzz-tie
```

Three things follow, all free.

`claude` keys on the directory name, still. The list has `t24b-dirname-only` and does not have
`t24b-frontmatter-name`.

`claude` does not dedupe on the frontmatter name. `t24b-aaa-tie` and `t24b-zzz-tie` both declare
`name: t24b-tie` and both are listed, where `agy` collapsed them to one.

`claude` does not drop a skill over the `: ` in its description. `t24b-desc-colon` is in the list.
Whether its description survives is the paid question below.

`cursor-agent` has no equivalent free route. It rejects an unknown model before writing any session
record, so its `store.db` only exists after a real run.

## Claude Code drops descriptions too, and by a different rule

One paid `claude` run, `--model haiku`, `$0.0228`. Its transcript's `skill_listing` renders the
nine project entries like this:

```
- t24b-clash: T24B clash probe. Marker CLASH-USER-CLAUDEROOT.
- t24b-aaa-tie: T24B tie probe A. Marker TIE-AAA.
- t24b-alpha: T24B probe alpha. Marker ALPHA-STORE.
- t24b-desc-colon: T24B colon probe. label: value here. Marker COLON-STORE.
- t24b-desc-colon2: T24B colon probe two. Ratio 3:1 no space. Marker COLON2-STORE.
- t24b-desc-plain
- t24b-dirname-only: T24B identity probe. Dir t24b-dirname-only, frontmatter ... IDENT-PROJECT.
- t24b-repo-own
- t24b-zzz-tie
```

The colon is a non-event here. `t24b-desc-colon` keeps its description intact, `: ` and all, so
Claude Code does not parse the frontmatter with a strict YAML loader. The same file that vanishes
from `agy` and loses its description in Cursor is fine in Claude.

`t24b-clash` appears once, carrying the user-scope marker. User scope beats project scope, and the
project entry is silently absent. That reproduces on `2.1.257`, which is the same build the
salvaged evidence ran on.

But three entries render as a bare name with no description at all, and it is not only my probes.
Across the whole listing, 14 of 35 render bare, including `handoff-frank`, `init`,
`security-review` and eight of the eleven `mattpocock-skills` entries. Two identical runs produced
byte-identical bare sets, so this is deterministic, not a race.

Compare the two runs against the same kind of worktree:

| run | model | skills | listing chars | bare entries |
|---|---|---|---|---|
| 2026-09-02, dead agent | `claude-opus-5[1m]` | 33 | 10,490 | 0 |
| 2026-09-03, this ticket | `haiku` | 35 | 7,987 | 14 |

More skills, a smaller listing. Claude Code is fitting the listing to a budget and paying for it by
stripping descriptions, and the only other thing that changed is the model. The next section pins
which.

### It is a character budget, and injection is what pushes people over it

Same model, same worktree, same run, with the three injected roots moved aside for one run and
moved back:

| skills present | listing chars | bare entries |
|---|---|---|
| 35, injection on | 7,987 | 14 |
| 26, injection off | 7,991 | 8 |

Nine more skills bought four more characters. Claude Code fills a fixed budget of roughly 8,000
characters with `name: description` lines and renders everything that does not fit as a bare name.
The `claude-opus-5[1m]` run got 10,490 characters and lost nothing, so the budget scales with the
model rather than being one constant.

The six extra bare entries are the finding. Three are Hydra's own links. The other three are
`mattpocock-skills:diagnosing-bugs`, `mattpocock-skills:tdd` and `mattpocock-skills:prototype`,
which had descriptions before the injection and lost them after it.

**Injecting skills into a Claude Code run silently strips descriptions from skills the user already
had.** Nothing warns. The skill is still listed and still invocable by name, so a human reading a
`/skills` output sees no problem, but the model loses the only text that tells it when to reach for
that skill. On a small-context model the budget is tight enough that nine links is enough to do it.

I did not work out which entry gets evicted first. It is not purely positional and not purely by
description length: `daily-agile-planner`, with the longest description in the set, keeps it, while
`handoff-frank` and `mattpocock-skills:writing-for-agents` both go bare. That rule is worth a
follow-up only if ticket 10 wants to guarantee Hydra's own skills survive.

What ticket 10 should take from it: the run record's `skills.linked` is not enough to reproduce
what a run's model actually saw. Two runs on the same store sha and the same linked names get
different descriptions delivered depending on the model and on how many unrelated skills the user
happens to have installed that day.

## Cursor on 2026.09.02, and the colon mechanism nailed down

The first paid Cursor attempt died on an account usage limit, not a token cost:

```
ActionRequiredError: You've hit your usage limit ... Switch to a different model or set a
Spend Limit to continue with this model. Your usage limits will reset ... on 9/29/2026.
```

`gpt-5.4-nano-none` is behind that limit. `composer-2.5` is not, and one run on it cost 7,649 input
plus 8,875 cache-read plus 25 output. That is worth knowing before a future ticket schedules Cursor
probes: model choice can turn a Cursor run from cheap into impossible, and the failed attempt wrote
no `store.db`, so nothing is minable from a refused run.

On the new build, `t24b-desc-colon` renders exactly as it did on the old one:

```xml
<agent_skill fullPath=".../wt-run/.cursor/skills/t24b-desc-colon/SKILL.md" />
```

The hazard is not a probe artifact. The same run rendered one of the user's real skills the same
way:

```xml
<agent_skill fullPath="/Users/USER/.claude/skills/handoff-frank/SKILL.md" />
```

`handoff-frank` has been on this machine since 31 August. Its description is 656 characters and
contains `Use it ALSO from the other side, to resume: when they say`. Every Cursor session on this
machine has been offering that skill with no description, and nobody noticed.

Fed to a real YAML parser, the frontmatter blocks say why:

```
handoff-frank      YAML FAILS: mapping values are not allowed in this context at line 3 column 468
t24b-desc-colon    YAML FAILS: mapping values are not allowed in this context at line 3 column 37
t24b-desc-colon2   YAML OK
```

Column 468 of `handoff-frank`'s line 3 is the space after `to resume:`. So the rule is exact: an
unquoted YAML scalar may not contain `: `, the whole frontmatter block fails to parse, and Cursor
falls back to an element with a path and nothing else. Quoting the value would fix it. Nothing in
any of the three CLIs tells you to.

### Cursor does dedupe, within a scope

The salvaged note says Cursor "does not dedupe at all". That is too strong, and the old evidence
already contradicted it if you read the paths rather than the model's summary.

In this run `t24b-clash` was linked into all three project roots, `.cursor/skills`,
`.claude/skills` and `.agents/skills`. It appears once, from `.cursor/skills`. All eight store
skills appear once each, all from `.cursor/skills`, and the only `.claude/skills` entry in the whole
listing is `t24b-repo-own`, which exists in no other root. Nothing from `.agents/skills` appears at
all.

The old run said the same thing: `t24-rootclash` existed in both `wt-run1/.claude/skills` and
`wt-run1/.cursor/skills`, and the listing carried it once, with the `ROOTCLASH-CURSORROOT` marker.

So Cursor collapses a duplicate across roots at the same scope and prefers `.cursor/skills`. What it
did not collapse, on the old build, was a duplicate across scopes: `~/.cursor/skills/t24-clash` and
`<project>/.cursor/skills/t24-clash` were both listed, under the same name, with different markers.

Re-tested on `2026.09.02` with `~/.cursor/skills/t24b-clash` present alongside the project link, and
it still lists both:

```
/Users/USER/.cursor/skills/t24b-clash/SKILL.md              | Marker CLASH-USER-CURSORROOT
.../wt-run/.cursor/skills/t24b-clash/SKILL.md                | Marker CLASH-PROJECT
```

The user-scope directory was created for that one run and removed immediately after it.

## Precedence, all three providers, current builds

| | `claude 2.1.257` | `cursor-agent 2026.09.02` | `agy 1.1.24` |
|---|---|---|---|
| identity compared | directory name | full path, no name field exists | frontmatter `name` |
| same name, user vs project | user wins, project dropped | neither, both listed | project wins, user dropped |
| same name, two roots, same scope | not testable, one root | `.cursor/skills` wins, others dropped | not testable, one root |
| two dirs, same frontmatter `name` | both listed | both listed | alphabetically first dir wins |
| description contains `: ` | kept intact | skill kept, description emptied | skill dropped entirely |
| long description | dropped whole when over budget | hard cut at 207 chars plus `...` | delivered whole |
| loser reported | no | no | no |

Every drop in that table is silent. No provider writes a warning to stdout or stderr, and every
probe run exited 0.

## Where Hydra can read the outcome

No provider reports a collision. All three expose the resolved listing after the fact, and two of
those routes cost nothing.

`agy`, free and before the run: `agy -p "/skills" --output-format json --add-dir <worktree>`
returns `num_turns: 0`, `total_tokens: 0`, and one record per surviving skill with `name`,
`description`, `path`, `builtin`, `model_invocable` and an optional `plugin`. Its
`command.data` object holds only `skills`. There is no diagnostics field and nothing names a loser.

`claude`, free and before the run: the bogus-model trick above writes a `skill_listing` attachment
carrying `skillCount` and a `names` array, without an API call. Descriptions are absent on that
path, names are not.

`cursor-agent`, paid and after the run: `~/.cursor/chats/<hash>/<session>/store.db`, largest blob,
`content` field, the `<available_skills>` block. Full paths and descriptions, so this is the only
route of the three that shows a dropped description as well as a dropped name. A run refused for a
usage limit writes no `store.db`.

So the check ticket 10 wants is buildable and mostly free: link the farm, ask the provider what it
sees, and compare against `skills.linked`. A name Hydra linked that is missing from the answer is a
collision, whichever direction the provider resolved it. That is a stronger check than comparing
strings, because it asks the provider instead of modelling it.

## `~/.grok/skills` and `~/.codex/skills`, closed

Nothing was established about these. They are real, at both scopes, for `cursor-agent` and for
nobody else.

Project scope, from the dead agent's run-3 transcript: `wt-run1/.codex/skills/t24b-only-codex`,
`wt-run1/.grok/skills/t24-only-grok` and `wt-run1/.agents/skills/t24-only-agents` each appeared,
each existing in exactly one root.

User scope, tested this session with one probe skill in each of `~/.grok/skills`, `~/.codex/skills`
and `~/.agents/skills`, all three created for one run and removed after it:

```
/Users/USER/.codex/skills/t24b-user-codex/SKILL.md   | Marker USERROOT-CODEX
/Users/USER/.grok/skills/t24b-user-grok/SKILL.md     | Marker USERROOT-GROK
/Users/USER/.agents/skills/t24b-user-agents/SKILL.md | Marker USERROOT-AGENTS
```

`agy 1.1.24`, asked in the same window for free, saw none of the three. Its user root is
`~/.gemini/config/skills` and nothing else, confirming ticket 10's claim that `~/.agents/skills` is
Cursor-only at user scope.

So `cursor-agent` reads eight skill roots, six of them belonging to other vendors: `.cursor/skills`,
`.cursor/skills-cursor`, `.agents/skills`, `.claude/skills`, `.codex/skills` and `.grok/skills`,
each joined to both the project directory and `homedir()`. Ticket 10 already knew this from binary
strings. It is now fired at both scopes.

Two consequences for the collision check. Hydra's user-scope index has to scan `~/.grok/skills`
and `~/.codex/skills`, neither of which exists on this machine today and either of which appears
the day the user installs the matching CLI. And a Cursor run inherits any `.codex/skills` or
`.grok/skills` the user's repo carries, which Hydra neither wrote nor sees.

## What ticket 10 got wrong, and what it should say instead

Ticket 10's Q10 says the launch-time collision check must "compare the same thing the provider
compares". Three problems with that sentence, in increasing order of how much they hurt.

There is no single thing. `claude` compares the directory name, `agy` compares the frontmatter
`name`, and the two disagree in the same file: `t24b-dirname-only` is one skill under two names
depending on who is looking.

Cursor compares nothing. Its listing has no name field, only a path and a description. It collapses
duplicates across roots at one scope and keeps duplicates across scopes, so its notion of identity
is the path with a root preference on top, not a name at all.

Comparing strings is the wrong shape of check anyway. All three providers will tell Hydra what they
resolved, and two of them for free. Ask, then diff against `skills.linked`.

So the sentence should read: **the check compares both the directory name and the frontmatter name
against every scope and every root that provider reads, and then confirms the result by asking the
provider for its resolved listing and diffing that against the names Hydra linked.** A name Hydra
linked and the provider does not list is a collision, regardless of which side won.

Two hazards ticket 10 does not mention at all, both of which fail a run silently:

A description containing `: ` breaks the skill on two of three providers. `agy` drops it, Cursor
empties its description, `claude` is fine. The store needs a write-time validator that runs a real
YAML load over the frontmatter, not a regex. One skill already installed on this machine,
`handoff-frank`, fails it today.

Injecting into `.claude/skills` strips descriptions from skills the user already had, because
Claude Code fits the whole listing into a character budget that varies with the model. Nine links
took six other skills from described to bare. Ticket 10's run record cannot reproduce what the model
saw from `skills.store_sha` and `skills.linked` alone.

## Costs

| Provider | Runs | Cost |
|---|---|---|
| `agy` | 3 | zero, `num_turns: 0` and `total_tokens: 0` every time |
| `claude`, free oracle | 1 | zero, rejected before the API call |
| `claude`, paid | 3 | `$0.0228`, `$0.0141`, `$0.0226`, all `--model haiku`, `$0.0595` total |
| `cursor-agent` | 4 | 1 refused on a usage limit, 3 at roughly 7,650 input plus 8,875 cache-read each |

The dead agent's single `claude` run cost `$0.1253` on `claude-opus-5[1m]`. Haiku is five times
cheaper and, for reading a skill listing out of a transcript, identical in what it returns. The one
thing to watch is the budget effect above: a cheap model gives a smaller listing, so the transcript
you mine is not the transcript an expensive model would have got.

## What the CLIs wrote, and what was cleaned up

Created and removed, all verified absent at the end:

- `~/.claude/skills/t24b-clash`, one directory, present for a single paid run
- `~/.cursor/skills/t24b-clash`, plus `~/.cursor/skills` itself, twice, each for one run
- `~/.gemini/config/skills/t24b-clash`, plus `~/.gemini/config/skills` itself
- `~/.grok/skills/t24b-user-grok` and `~/.grok`
- `~/.codex/skills/t24b-user-codex` and `~/.codex`
- `~/.agents/skills/t24b-user-agents` and `~/.agents`

`~/.claude/skills` is back to its 11 entries and holds nothing matching `t24`. None of
`~/.cursor/skills`, `~/.gemini/config/skills`, `~/.agents`, `~/.codex` or `~/.grok` exists.
Nothing under `~/hydra` outside `.scratch/hydra/` was touched, and no settings, hook or MCP file
was read or written.

Left in place on purpose, because they are the providers' own session bookkeeping and a resumed
session can mine them for free:

- `~/.claude/projects/-private-tmp-...-scratchpad-t24b-wt-run/*.jsonl`, four transcripts
- `~/.claude/projects/-private-tmp-...-scratchpad-t24b-userrepo/memory/`, empty
- `~/.cursor/projects/private-tmp-...-scratchpad-t24b-wt-run/`, with `mcp-auth.json`,
  `repo.json`, `worker.log`, `.workspace-trusted` and `agent-transcripts/`
- `~/.cursor/chats/ef00342f386f0d37530f9b4bdadaaae4/`, three `store.db` files
- `agy` touched `~/.gemini/antigravity-cli/` bookkeeping and wrote no skill directory

The probe rig is at
`/private/tmp/claude-501/-Users-USER/f2c096c9-40e2-4c31-bdb1-bdcf8d1f4683/scratchpad/t24b/`, store
sha `ae43af01`. It is session scratch and will not survive long.
