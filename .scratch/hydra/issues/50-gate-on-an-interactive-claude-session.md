# 50 - Can the gate ride an interactive `claude` session?

Type: prototype
Status: resolved
Blocked by:

## Question

Harness spec 3 gives an interactive run no gate, no skills and no preamble: the vendor owns the
terminal and Hydra gets out of the way, and the record is harvested thin at exit. Ticket 38 added
the reason the gate is absent: what fires on an interactive session is the user's own hooks, and
those were never Hydra's to record a decision on. The human asked on 2026-09-07 what the best of
both worlds is, since an interactive session is the only producer of human corrections and today
the only mode that runs without Hydra's rules.

`claude` accepts `--settings <file>` in interactive mode, and Hydra's gate is nothing but a
`PreToolUse` hook in that file. So the question is measurable: **launched interactively with
Hydra's settings and inside a worktree, does the gate refuse what the policy denies, and do the
hook's decisions land in the session transcript under `~/.claude/projects/` that the harvest
reads?** A yes on both makes an interactive `claude` run a recorded run with a human at the
keyboard, and turns fase 2's trade into a per-provider limit rather than a mode. A no on the
second keeps spec 3 as written.

## Probe

1. Compile a `read-only` gate into a scratch spawn directory the way `hydra run` does, and write
   the same `settings.json`.
2. In a Herdr pane, `herdr agent start probe --kind claude -- --settings <that file>`, cwd a
   scratch worktree of this repo. Prompt it to write a file, then to read one.
3. Read the pane: was the write refused with Hydra's reason? Then read the transcript file for
   that session: does it carry `hook_started`, `hook_response` or any frame naming the decision?
4. Same session, check the skill listing with the snapshot linked into the worktree.
5. Record `claude --version` and the date.

## Where the evidence is

Harness spec 3 and 10, ticket 38's `## Answer`, `src/provider/claude.rs::settings`, the harvest
shapes in contract 7.2.1.

## Answer

Probed 2026-09-07, `claude` 2.1.263, Herdr 0.8.2, from the orchestrating session through
`herdr agent start --kind claude -- --settings <file> --session-id <uuid>` in a pane whose cwd
was a scratch worktree of this repo. The settings file and the gate were the ones a real
`read-only` launch had just compiled, copied out of its spawn directory with the path inside the
hook command rewritten and the sha untouched. The session ran the human's default model, Fable
5.1, with the human's own hooks loaded beside Hydra's.

**The gate applies.** Asked to write `probe50.txt`, the session reported: "The Write call was
refused with the message: hydra/capability: policy read-only does not allow file_write
(Write)." Asked to read `CLAUDE.md`, it was refused with `hydra/out-of-scope`, because the gate
had been compiled for the other run's worktree and the probe's cwd was outside it. Two calls,
two of Hydra's reasons, on a session a human was steering.

**The decisions reach the transcript, but only the denials, and not as frames.** The session
file under `~/.claude/projects/` holds eleven record types and no `hook_started` or
`hook_response` among them. Each refused call is there as a `user` record whose `tool_result`
carries `is_error: true` and the reason string verbatim, `hydra/capability: ...` and
`hydra/out-of-scope: ...`, with `toolUseResult` repeating it prefixed `Error:`. An allowed call
leaves nothing that says the gate saw it. So a harvester can derive a `permission_decision` of
`deny` from a `tool_result` whose text starts with `hydra/`, and it can derive nothing for
`allow`: absence means the gate said yes or the gate was not there, and the record cannot tell
which without `run_start.policy` saying a gate was compiled.

**What this makes possible.** An interactive `claude` run launched by Hydra with its own
settings and its own worktree is a gated run with a human at the keyboard: refusals are real and
recorded, the policy is named on the envelope, skills can be linked into the worktree, and the
transcript streams live so the pane can grow a graph. What it is not is a recorded run: no
`allow` decisions, so no gate-leak check, and rung one's `launch_mode == recorded` filter keeps
it out of the skill lessons, which is right. Harness spec 3's "no gate" was a choice and not a
limit of the binary, and it can be revisited per provider: `cursor-agent` and `agy` read their
hooks out of a worktree, which an interactive run of theirs would then need too.

**Decision left to the human.** Whether `hydra run --interactive` on `claude` takes `--policy`
and compiles a gate, with the record's `policy` set and `permission_decision` derived for
denials only. Build ticket 27 lands the ungated version first, as the spec says, and this is the
next question on it.

**Decided 2026-09-07 with the human, in the same session.** `hydra run --interactive` takes
`--policy` as an option and has no policy by default. Without it the launch is what spec 3
says: the human's own session, no gate, no skills, no worktree, a thin record harvested at exit.
With it, Hydra compiles the gate, creates and seeds a worktree, links the snapshot, hands the
provider the same settings a recorded run gets, and the record carries the policy and its sha,
`permission_decision` for every denial the transcript holds, and a `Finding::Limit` saying that
allows were not recorded on this launch mode. `claude` only, since the other two read their
hooks out of a worktree they do not get. Harness spec 3 carries the paragraph; build ticket 28
carries the code and waits on 27.
