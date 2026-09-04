# 10 - Skill store layout and injection

Type: grilling
Status: resolved
Blocked by: 03, 05

## Question

D5 makes the skill store the second thing both systems share, and D9 fixes the broad approach. This ticket makes it precise.

Settle:

- The layout of `~/hydra/skills/`: flat or nested, and what metadata sits alongside each `SKILL.md`
- What "inject" means per provider, given the discovery rules ticket 03 found: symlink, generated wrapper, or copy
- What happens to the skills already living in `~/.claude/skills`, `~/.cursor/skills-cursor` and `~/.hermes/skills`. Adopted into the canonical store, left in place, or shadowed
- How a skill declares which providers it applies to, if some are provider-specific
- Which skills Ouroboros may edit and which are off limits, which is the pin concept from ticket 05
- How a change propagates: immediately through symlinks, or on the next `hydra` launch

Scenario to test against: Ouroboros rewrites a skill mid-session while three provider CLIs are running. Who sees the old version, who sees the new one, and is that acceptable?

## What the research changed

- **D9's Cursor path was wrong.** `~/.cursor/skills-cursor` is Cursor's managed cache of its own built-ins. The personal path Cursor scans is `~/.cursor/skills/`, which does not exist on this machine yet.
- **One `SKILL.md` serves all four providers unchanged**, carrying only `name` and `description`. No frontmatter field forces a generated wrapper, so injection can stay a symlink farm.
- **Two gaps have to close before this ticket can specify injection.** Claude Code follows symlinked skill directories, documented and proven live. Hermes does, from its glob semantics. Whether Cursor and Gemini follow symlinks is unconfirmed, because ticket 03 was read-only against live config. If either does not, that provider needs a copy step and the propagation question changes with it.
- **Gemini's plugin path is a version detail, not a contract.** `~/.gemini/config/plugins/` already diverges from every public doc and carries a `.migrated` marker.

## What the symlink test found

Both gaps above are closed, and the answers are not symmetrical.

- **Gemini follows a symlinked skill directory** and ignores a symlinked `SKILL.md` inside a real
  one, silently. So injection links directories, never files. Its personal path is
  `~/.gemini/skills/<name>/SKILL.md` with no plugin wrapper, which makes D9's "plugin-wrapped for
  Gemini" wrong at 0.42.0.
- **Cursor has no skills concept** at the installed 2025.10.02 build. It reads `.mdc` rules found by
  walking up from the project root. So the ticket's injection question forks for Cursor: convert to
  `.mdc`, or leave Cursor out of the shared store until it is updated.

Detail, method and the two things still unverified in
[research/10-symlink-resolution.md](../research/10-symlink-resolution.md).

## What the second evidence pass found, 2026-09-01

Appended under `# 2026-09-01, second pass` in
[research/10-symlink-resolution.md](../research/10-symlink-resolution.md).

- **The Cursor fork above is dead.** `cursor-agent` self-updated at 12:10 on 2026-09-01, from
  2025.10.02 to 2026.08.31-4057e58, and the new build has skills. The section above that says
  "Cursor has no skills concept" was accurate the day it was written and is false now. Do not grill
  against it. One trap: there is still no `skills` subcommand, so grepping `--help` is not a test
  for whether the binary has skills.
- **The provider locations are not independent.** Cursor's skill roots are `.cursor/skills`,
  `.agents/skills` and `.cursor/skills-cursor`, plus `.claude/skills`, `.codex/skills` and
  `.grok/skills` behind a third-party flag that is on by default here, each joined to both the
  project directory and `homedir()`. So `~/.claude/skills` is already a Cursor root, and injecting
  for Claude Code injects for Cursor at the same time. This ticket's injection question is not four
  independent targets, it is an overlapping set.
- **Gemini does follow a symlinked `SKILL.md`.** The first pass said it ignores one silently and
  gave a `Dirent.isFile()` mechanism for why. Both wrong. The original error came from a relative
  link at the wrong depth dangling quietly, plus Gemini deduping on frontmatter `name`.
- **Ticket 03's Hermes reasoning was wrong, with the right conclusion.** Hermes follows symlinks
  through `os.walk(..., followlinks=True)`, not the `rglob` semantics ticket 03 reasoned from.
  `rglob` on Hermes' own Python 3.11.15 misses symlinked directories, tested.
- **The propagation scenario has a mechanical answer now.** Hermes re-reads a skill from disk on
  every call. Cursor and Gemini both cache the body at discovery and stay frozen until session
  start. Claude Code's metadata is fixed at session start. A mid-session rewrite splits two against
  one. Whether that is acceptable is still this ticket's question.
- **Eight name collisions**, all between `~/.claude/skills` and the mattpocock plugin cache, all
  carrying `disable-model-invocation`, all predating Hydra.
- D9's "plugin-wrapped for Gemini" is re-confirmed wrong at gemini 0.42.0.

## What the third evidence pass found, 2026-09-02

Gathered live this session, before grilling. No research file: every claim below is one command.

- **`agy` has a skills concept and its own personal root.** Strings in the 1.1.23 binary give
  `<workspace>/.agents/skills/<name>/` and `~/.gemini/config/skills/<name>/SKILL.md`. Built-ins live
  in `~/.gemini/antigravity-cli/builtin/skills/` behind a `.checksum`, which is the same managed
  cache pattern as `~/.cursor/skills-cursor`. There is no `skills` subcommand, the same trap
  `cursor-agent` set.
- **`agy` follows a symlinked skill directory.** Probed live: a link at
  `~/.gemini/config/skills/hydra-symlink-probe` pointing outside the tree was discovered, and the
  reported `path` is the link path, not the target. Link and probe both removed afterwards;
  `~/.gemini/config/skills/` did not exist before and does not exist now.
- **`agy -p "/skills" --output-format json` is a free inventory oracle.** It returns
  `num_turns: 0` and `total_tokens: 0`, and each entry carries `name`, `description`, `path`,
  `builtin`, `model_invocable` and an optional `plugin`. Any injection design can be verified against
  it at zero cost. The changelog says `/usage`, `/quota`, `/credits`, `/model`, `/effort` and
  `/skills` all answer non-interactively in print mode.
- **All three routable providers follow symlinked skill directories**, so D9's symlink farm is
  mechanically sound everywhere. Claude Code from ticket 03, `cursor-agent` from the second pass,
  `agy` from the probe above.
- **The injection targets overlap, so there are two of them and not three.** Cursor's user-scope
  roots include `~/.claude/skills` with the third-party flag on by default, so one farm under
  `~/.claude/skills` reaches both `claude` and `cursor-agent`. `agy` needs a second at
  `~/.gemini/config/skills`. `~/.agents/skills` is Cursor-only at user scope: `agy` reads
  `.agents/skills` at the workspace, never at `homedir()`.
- **Hydra cannot give a run an isolated skill set.** Every provider reads a user-scope root as well
  as a project one, and ticket 21 rules out moving `HOME`, since a wrong one silently logs three of
  four providers out. So the user's personal skills are present in every run whatever this ticket
  decides, and the only question is whether the store manages them or ignores them.
- **The store is empty and so are two of the three targets.** `~/hydra/skills/` does not exist.
  `~/.claude/skills/` holds 11 skills. `~/.cursor/skills/` and `~/.gemini/config/skills/` do not
  exist.
- **Two providers dropped out of the question.** Ticket 22 unrouted hermes, so `~/.hermes/skills`
  is no longer an injection target. Ticket 18 replaced `gemini` with `agy`, so the first and second
  passes' Gemini findings, including the `~/.gemini/skills/` path and the plugin-wrapper argument,
  describe a binary v1 does not route.
- **`agy`'s workspace root needs `--add-dir`, not a cwd.** `<workspace>/.agents/skills/<name>/`
  loads, symlinked or real, but only when the directory is passed as `--add-dir`. Launching from
  inside it does nothing: a probe placed there and run from that cwd was invisible, twice, as a real
  directory and as a link. Passed by flag it appears, and it appears from an unrelated cwd too. This
  matches ticket 18's finding that `--dangerously-skip-permissions` resolved a relative path into
  `~/.gemini/antigravity-cli/scratch/` rather than the launch cwd. `agy` has a workspace concept
  detached from the process working directory, so any run-scope injection for Google is also a
  launch-flag contract for ticket 13.
- **A linked worktree has no private `info/exclude`.** Git reads only the shared
  `$GIT_COMMON_DIR/info/exclude`, tested: a pattern written to `.git/worktrees/<name>/info/exclude`
  did nothing, and the same pattern in `.git/info/exclude` worked but silenced it in the main
  working tree too. So the obvious way to hide injected links from the child's `git status` is a
  write into the user's repo that reaches beyond the run.
- **A `.gitignore` holding `*` inside the injected directory hides it with no such write.** Tested:
  `git status` reports nothing, `git add -A` stages nothing, and where the repo already tracks
  `.claude/skills/`, its own skill stays tracked and visible while only Hydra's link is ignored,
  because git never ignores a tracked file. `agy` still discovers a linked skill sitting beside that
  `.gitignore`, and Cursor's `Ur` filter drops any path component starting with a dot, so the file
  is invisible to discovery as well as to git.

## Answer

**One flat git-versioned store at `~/hydra/skills/`, injected per run into the worktree rather than
into your home directory, materialised as an immutable snapshot so a running session's bytes cannot
change underneath it.** D9's symlink farm survives as a mechanism and dies as a location.

### The store

`~/hydra/skills/<name>/SKILL.md`, flat. Nesting buys nothing: all three providers flatten to a name,
and Hermes was the only one that ever reported a category. Beside each `SKILL.md` sits an optional
`hydra.toml`, which only Hydra reads.

`SKILL.md` keeps exactly what ticket 03 proved portable, `name` and `description`. Hydra's own
metadata stays out of it. Cursor's `.mdc` parser is hand-rolled and splits each line on the first
colon, and Gemini dedupes on the `name` field, so vendor frontmatter parsers here are fragile and
they change under you. A sidecar cannot be misparsed by a provider that never opens it.

The sidecar carries three fields:

- `origin`, either `human` or `ouroboros`. Records authorship and never changes.
- `pinned`, a boolean. Yours to set. Blocks every write when true. Defaults to true for
  `origin = "human"` and false for `origin = "ouroboros"`.
- `providers`, an allowlist. Absent means every routable provider. A name Hydra does not route,
  which today means `hermes` or `gemini`, warns at launch and is otherwise ignored, so the store
  survives the provider set changing again.

`pinned` is deliberately separate from `origin` rather than derived from it. Derived, you could not
freeze a misbehaving Ouroboros skill without rewriting its authorship. Ticket 05's rule list pairs
the pin with a patch exemption, and **that exemption is dropped rather than defined.** Hermes needed
it because pinning was its only brake. Hydra has `git revert` over the store and a proposal path for
human-owned skills, so a pinned skill that needs a fix gets one from a human or from an unpinning. A
guarantee with an exception you can argue your way into is the failure ticket 05 is named after.

The store is **its own git repo**, living inside the `~/hydra` working tree and ignored by it. D2
keeps one repo because the harness and Ouroboros share a contract; the store is data both of them
write, not code, and mixing skill commits into the same history as rung two's source edits makes the
rung-two diff quorum harder to reason about. Versioning it is what pays for everything below:
ticket 05's "snapshot before every mutating pass" costs nothing because every run already left one,
rollback is `git revert`, and ticket 14's scenario of a skill rewritten worse three weeks running
becomes a join between git history and run outcomes.

It starts empty. Rung one's first act on any skill is therefore always a create, which is consistent
with `origin` being the thing that grants edit rights.

### What happens to the skills already on disk

Nothing moves. The 11 skills in `~/.claude/skills` stay where they are, and so do the plugin caches.

This is not generosity, it is the only honest option. **Hydra cannot give a run an isolated skill
set.** Every provider reads a user-scope root as well as a project one, and ticket 21 rules out
moving `HOME`, since a wrong one silently logs three of four providers out. So those skills are in
every run whatever this ticket decides. The real choice was never whether they load, only whether
the store knows about them.

Hydra scans the user-scope roots into a **read-only index** kept in the store, rebuilt at every
launch and before every curator pass. Ouroboros can see them, reason about them, and blame them for
a failure. It cannot write to them. A skill with no sidecar is pinned and unproposable by definition,
because Hydra does not own the file.

### Injection

**Recorded runs only.** An interactive run in a directory Hydra does not own gets no injection, and
the spec says so out loud rather than leaving it implied. Ticket 13 already found that interactive
runs record thin and are harvested at exit; on the skill store they are thin too. An interactive
session is a human driving with their own skills, and Hydra is a launcher there rather than a
harness.

For a recorded run, one worktree runs one provider, which is what makes the overlap harmless. Hydra
creates one directory in the worktree, using that provider's **first-party root only**:

| Provider | Root |
|---|---|
| `claude` | `<worktree>/.claude/skills/` |
| `cursor-agent` | `<worktree>/.cursor/skills/` |
| `agy` | `<worktree>/.agents/skills/` plus `--add-dir <worktree>` |

Cursor reads `~/.claude/skills` as a first-class user root, so one shared farm would have been
fewer moving parts. It is rejected because that read is gated on a settings flag which is merely on
by default, and a default is not a contract.

`agy`'s row carries a launch flag because a directory it is not told about is not its workspace. A
probe placed in the launch directory was invisible twice, as a real directory and as a symlink;
passed as `--add-dir` it appeared, and appeared from an unrelated cwd too. **Hydra passing the
worktree to `agy` is not an optimisation, it is the difference between the skills loading and not.**
That is a launch-contract change for ticket 13.

Into that directory Hydra drops a `.gitignore` holding `*`, then links in the skills whose
`providers` field admits this run. The worktree belongs to the user's code repo, so the alternative
was hiding the links through git's exclude machinery, and that turns out to be a trap: a linked
worktree has no private `info/exclude`, git reads only the shared one, so hiding the links from the
child would also hide them in the user's main working tree. The self-ignoring directory needs no
write outside the directories Hydra created. `git status` reports nothing, `git add -A` stages
nothing, and where a repo already tracks `.claude/skills/`, its own skill stays tracked and visible,
because git never ignores a tracked file. Cleanup is `rm -rf` on a path Hydra owns, and a crash
leaves litter only inside a worktree ticket 11 already throws away.

All three providers follow symlinked skill directories, so the farm itself is sound everywhere:
Claude Code from ticket 03, `cursor-agent` from the second pass, `agy` from this session's probe.

### Propagation, and the scenario

The links do not point at `~/hydra/skills`. They point into
`~/hydra/skills-snapshots/<sha>/`, a git worktree of the store checked out at the sha the run pinned.

Without that, versioning the store is decorative. A symlink resolves to the live working tree, so
Ouroboros editing a file would reach a session already running: Cursor and `agy` cache a skill body
at discovery, but Claude Code fixes only its metadata at session start, so a skill loaded later in a
long session reads whatever is on disk then.

Snapshots are **shared per sha and refcounted**, not per run. Ticket 11 caps delegation depth at 2
and does not cap fan-out, so twenty children on one sha would otherwise mean twenty identical
checkouts. Sharing also makes the guarantee structural instead of coincidental: two runs on the same
sha see identical bytes because they are looking at the same bytes. A snapshot is collectable when no
live run references it, and liveness is already a pid plus a process start time from ticket 13.

**If the store is dirty at launch, Hydra commits it before pinning**, with authorship in the commit
trailer so ticket 14 can still separate a human's edits from Ouroboros'. Launching from HEAD and
ignoring the working tree would mean editing a skill, running, and watching nothing happen, which is
the worst failure this design can produce and would happen on day one. Refusing to launch punishes
an edit that has nothing to do with the run. Committing is the only option where the sha in the run
record is always true.

So the ticket's scenario, Ouroboros rewriting a skill mid-session while three CLIs run: **nobody sees
it, and the question of whether that is acceptable dissolves.** Each session is linked into an
immutable snapshot at its own sha. The rewrite lands as a commit and reaches the next run. The
provider-by-provider caching table from the second pass stops mattering, because it only described
what happens when the bytes can change, and now they cannot.

### Collisions

The second pass found eight, all between `~/.claude/skills` and the mattpocock plugin cache, all
predating Hydra. Recording them at launch was the first answer and it was wrong, because it accepted
as unavoidable something Hydra can prevent.

Two checks, at two different times:

- **At write.** The curator refuses to create or rename a store skill to a name the index already
  holds. Hydra knows every taken name, because the index exists for exactly this. No collision Hydra
  causes can reach a run.
- **At launch.** Hydra compares the store against the index and against the repo's own project-scope
  skills, and a hit **fails the run**, as its own outcome in ticket 12's shape rather than a field
  buried in the record.

Failing looks harsh until you notice what the write check did to the odds. A launch-time collision no
longer means Hydra was careless, it means the world moved under the store: you added a skill, or a
plugin did. That should be loud, it should never fire, and the fix is one rename.

### What the run record carries

`skills.store_sha`, the commit the snapshot was checked out at, and `skills.linked`, the names
actually linked for that provider. Neither is sufficient alone. The sha does not determine the skill
set, because `providers` means two providers on one sha get different skills; the name list does not
determine the bytes. Together they reproduce a run's skill set exactly. Ticket 09 owns the schema;
this is input to it.

The index sha stays out. It describes the user's home directory rather than the run, and the only
thing it would explain is now a run outcome.

### Enforcement

Every limit above lives at the curator's tool-dispatch layer, which refuses any write path outside
the store, and none of it lives in a prompt. That is ticket 05's lesson applied rather than restated:
Hermes told its model that archiving was recoverable while the tool it called ran `shutil.rmtree`.

### What this changes elsewhere

- **D9 is wrong on both halves** and is amended on the map. Injection is per-run into the worktree,
  not a farm in the vendor locations, and Google needs no plugin wrapper.
- **Ticket 13** gains `--add-dir <worktree>` as a launch requirement for `agy`.
- **Ticket 09** gains `skills.store_sha` and `skills.linked`.
- **The MCP fog patch** was waiting on this ticket and can now be specified. Graduated as ticket 23.
- **The verification gaps this design creates rather than closes** are ticket 24, which builds the
  injection shape and watches all three providers load it, and ticket 25, which settles the Cursor
  usage fact ticket 09 rests on.

## Amended by ticket 24, 2026-09-03

Built the injection shape against `claude 2.1.257`, `cursor-agent 2026.09.02` and `agy 1.1.24` and
watched all three load it. Evidence in
[research/24-verify-injection-and-precedence.md](../research/24-verify-injection-and-precedence.md).
The answer above stands and is not edited. Four things it gets wrong or leaves out.

**Q10's identity sentence is wrong.** It says the collision check must "compare the same thing the
provider compares". There is no such thing. `claude` keys on the directory name, `agy` keys on the
frontmatter `name`, and one probe directory named `t24b-dirname-only` declaring
`name: t24b-frontmatter-name` is a different skill to each of them. Cursor compares neither: its
listing has no name field at all, only a full path and a description, so its identity is the path
with a root preference layered on. What the check must compare instead: **both the directory name
and the frontmatter name, against every scope and every root that provider reads, and then the
provider's own resolved listing diffed against `skills.linked`.** A name Hydra linked that the
provider does not list is a collision, whichever side won. Two of the three listings are free, so
asking is cheaper than modelling.

**A description containing `: ` breaks the skill on two of three providers, silently.** An unquoted
YAML scalar may not contain a colon followed by a space, so the frontmatter fails to parse. `agy`
drops the skill from its listing. Cursor keeps it and emits a path with an empty description.
`claude` is fine. The store needs a write-time validator that runs a real YAML load over the
frontmatter, not a regex, and this is a stricter rule than "it looks like valid frontmatter".
`~/.claude/skills/handoff-frank` fails it today and has been reaching Cursor with no description
since 31 August.

**Injection strips descriptions from the user's existing skills on `claude`.** Claude Code fits the
whole listing into a character budget that scales with the model and renders the overflow as bare
names. One run with Hydra's nine links and one without, same model and worktree: 7,987 characters
against 7,991, bare entries 14 against 8. Three of the six newly stripped were the user's own plugin
skills, and nothing warned. So `skills.store_sha` plus `skills.linked` does not reproduce what a
run's model saw, and the injection is not as free of side effects as the answer above assumes.

**The Cursor collision surface is two roots wider than the answer accounts for.** `cursor-agent`
reads `.codex/skills` and `.grok/skills` at project scope and at `homedir()`, both now fired live.
Neither exists on this machine today and either appears the day the matching CLI is installed. The
user-scope index has to scan both, and a Cursor run inherits any `.codex/skills` or `.grok/skills`
the user's repo carries, which Hydra never wrote and does not check.

Two claims in the answer survive intact. Q14's shape works end to end: `git status` clean,
`git add -A` staging nothing, the repo's own tracked project skill still visible beside the ignored
links. And failing the run on a launch-time collision is right, because the three providers resolve
one in three incompatible directions, so no rule about who wins could be written into the spec.

## Narrowed by ticket 14, 2026-09-03

**`origin = "ouroboros"` is not editable in place in v1.** This answer made an Ouroboros-authored
skill writable without a human seeing it. Ticket 14 closes that for now: rung one commits every
change to a branch in the store and a human merges it, whoever wrote the skill originally.

The sidecar fields are unchanged and `origin` still grants the right. What changes is the default:
the right is not exercised until a graduation condition is met, and the count is picked from the
corpus rather than now. Everything else here stands.

## Amended by ticket 23, 2026-09-03

**The self-ignoring directory trick does not cover MCP.** This ticket's injection shape hides each
first-party root from git with a `.gitignore` holding `*`. `.cursor/mcp.json` and
`.agents/mcp_config.json` sit inside such a root and are covered. Claude's `.mcp.json` sits at the
worktree root and cannot be, so `git status` shows it and `git add -A` stages it into the child's
branch. Ticket 29 decides whether Claude's config goes there at all.

**Symlink behaviour is per loader, not per provider.** Ticket 26 found `cursor-agent` silently
refuses a symlinked hook config. Ticket 23 found the same binary follows a symlinked MCP config,
absolute or relative, confirmed in a paid run. So this ticket's symlink injection survives for
skills and for MCP, and ticket 26's "real files, never links" stays scoped to hook config rather
than becoming a general rule.

## Amended by ticket 29, 2026-09-03

**`claude` leaves this shape for MCP, and only for MCP.** Its MCP config goes on the command line,
`--mcp-config <file in the run's spawn directory>` plus `--strict-mcp-config` on recorded runs and
without the strict flag interactively. So the git problem ticket 23 found never arises: no
`.mcp.json` appears at the worktree root, `git status` stays clean, and the ancestor merge stops
existing rather than being defended against by choosing where worktrees go. Skill injection is
untouched, and MCP still rides this shape on `cursor-agent` and `agy`.

**Hydra now writes in three places per run**, and this ticket's rule still holds in all three: the
worktree it created, the immutable snapshot at `~/hydra/skills-snapshots/<sha>/`, and a new spawn
directory at `~/hydra/spawn/<run-id>/` holding what Hydra generated at launch. Ticket 29 kept the
rule true on Cursor, where ticket 23 thought the vendor had broken it, by redirecting
`CURSOR_DATA_DIR` into that spawn directory.
