# 261 - Master of Puppets on any repository

Type: build
Status: resolved
Blocked by:

## What

Asked by the human on 2026-10-03: a script that opens a Master of Puppets orchestration session
on any repository, with the cockpit, rather than a bare `claude` in that repository. The
`puppets` skill lives in `~/hydra/.claude/skills/puppets/`, a project skill, so `/puppets` existed
only in sessions started inside `~/hydra`.

## Outcome

Built by the session on 2026-10-03, no agent. `hydra-copilot` reads `HYDRA_COPILOT_MODE`
(`copilot` by default, `puppets`, anything else exits 1). In `puppets` mode it starts from
`~/.hydra-puppets-copilot/`, renders `COPILOT.md` as before, appends the new
`integrations/copilot/PUPPETS.md` and the skill's body (frontmatter stripped) to `CLAUDE.md` and
`AGENTS.md`, and copies the skill to `.claude/skills/puppets/SKILL.md` there, with `{{REPO}}`,
`{{HYDRA}}` and `~/hydra/` rendered. `PUPPETS.md` says every `hydra-puppets` command runs as `cd
<repo> && ...`, since `hydra-puppets` launches from its own working directory
(`crates/puppets/src/launch.rs`), and that the loop's approvals are the person's word the
copilot's no-commit rule asks for. `~/hydra-oficina/masterofpuppets.sh <repo> [model] [provider]`
is the wrapper, outside the repository.

Fixed on the way: `COPILOT.md` (build 253) pointed the chat at `{{REPO}}/COCKPIT.md`, which a
repository other than Hydra does not have; it now renders Hydra's checkout.

Test: `hydra_copilot_in_puppets_mode_installs_the_skill_and_the_loop` in
`tests/copilot_repo_and_tools.rs` (both files carry the mode and the loop with no placeholder
left, the skill is installed, an unknown mode exits 1). Checked by hand against a stub `hydra`
with `~/private-repo`. Not checked: a real session typing `/puppets` from that start directory,
which may first ask the human to trust the directory.

Moved the same day: `~/hydra-oficina/`, the office root of the 2026-10-02 usability probes, was
deleted at the human's word. Its four run records went to `~/hydra/pruned/` with the other 34,
its four worktrees were reaped first (branches kept), and its scripts were dropped except two:
`integrations/copilot/bin/masterofpuppets` in the repository, and the links `masterofpuppets` and
`suprema` (to `hydra-copilot`) in `~/.local/bin`. Both scripts now resolve their own symlink with
`readlink -f`, because through a link `dirname "$0"` named `~/.local/bin`; checked through both
links against a stub `hydra`.
