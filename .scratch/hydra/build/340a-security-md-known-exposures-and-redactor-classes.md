# Build 340a - `SECURITY.md` states the twelve redactor classes, `~/.claude.json`, and the public-gate exposures

Status: resolved
Size: haiku

## Goal

Update `SECURITY.md` only so that every security claim in the repository's public security document matches the code at build 361 and states the open exposures from map 134 in plain words. Do not touch any other file except `SECURITY.md` and this ticket file. Follow `~/.claude/skills/unslop/SKILL.md`: no em dashes in new prose.

## What to change in `SECURITY.md`

1. **Lines 78 to 80 (`~/.claude.json`, map ticket 99):**
   After `which writes nothing under your home directory outside its worktree and claude's session state.`, add:
   `That state includes ~/.claude.json, where Claude Code keeps user-scope MCP servers, and ~/.claude/history.jsonl*. A gated interactive claude session can add a server in ~/.claude.json, and the next ungated claude session starts it, map ticket 99.`

2. **Lines 146 to 149 (unhooked vendor tools):**
   Extend the bullet so it also names `codex`: on `codex`, hosted `WebSearch` runs without consulting the hook; under a policy that denies `network_read` it cannot be blocked before it runs and is recorded as `UnhookedDenied` at normalization, while `webrun` is hooked since build ticket 338. Also note that on `claude` 2.1.301 `LSP` is unknown and refused and `Workflow` maps to `subagent_spawn` by reading, build ticket 350.

3. **Lines 167 to 175 (`hydra show` marking):**
   Note that since build ticket 356 `hydra show --events` also wraps `gate_refusal.reason` in `<<<hydra-result TOKEN>>>` and `<<<end hydra-result TOKEN>>>`, because `gate.py` interpolates the refused token into `reason` and the sidecar is writable inside the run's seatbelt.

4. **Lines 283 to 302 (`codex` and `claude` hooks and `/usr/bin/python3`):**
   Add that since build ticket 358 `hydra doctor` runs `/usr/bin/python3 -c "import json, shlex"` so a missing or broken system Python is reported before launch, and that `hydra doctor` reports `claude` and `codex` as unmeasured when the installed binary moves past `MEASURED_VERSIONS`.

5. **Lines 307 to 320 (`gate_refusal` rows):**
   Update the bullet to state that since build ticket 339 the hook appends a `gate_refusal` row for every denied call, not only shell commands, carrying `arguments_sha256` when there is no command; that `reason` still names a refused path; that a deny whose payload has no string `tool_name` writes no row; and that since build ticket 360 a `gate_refusal` row in Hydra's voice derives `Outcome::Gated` when no call got through.

6. **Lines 322 to 330 (export redactor and write-time secret sweep):**
   Replace the stale claim that `hydra export` redacts six classes and omits AWS keys and Slack tokens:
   - State that since build ticket 128 `hydra export` redacts twelve classes of secret, including AWS `AKIA` access keys, `aws_secret_access_key=` values, Slack `xoxb-` bot tokens and `hooks.slack.com` webhooks, and GitHub `github_pat_` and `gho_` tokens, alongside OpenAI keys and PEM headers.
   - State that JWTs are not a named class, that the write-time sweep in `src/record/secrets.rs` still flags the original five classes, map ticket 108, and that a secret split by a space or URL-encoded reaches neither check.

7. **Add two bullets under `## Known exposures, stated before someone else does`:**
   - **`git push` floor and its remaining shell limits, builds 349 and 357:** Since build tickets 349 and 357 the pattern floor refuses a bare `git push` from a protected branch, `git push origin HEAD` and `@`, `git -C`, `--git-dir` and `--work-tree` before `push`, `git -c` and `--config-env` push refspecs, and `refs/heads/` branch prefixes. A `cd` earlier in the same shell command and `push.default=matching` in a repository config still reach a protected branch, because the gate does not interpret shell or read repository git config.
   - **What remains open across map 134:** State concisely that the interactive gate on `cursor-agent`, `agy` and `codex` is unmeasured and refused until build ticket 130; that Linux compiles and runs CI since builds 315 and 331 but launches hook-only without an OS sandbox until build ticket 318 lands; that `hydra doctor --walk` walks the live corpus on the author's machine and not in CI; that `hydra bench` counts a `completed` run as a successful attack, map ticket 114; and that nothing sends diffs off the machine today, with map ticket 110 governing any future return of `jev ask`.

## Done when

- Only `SECURITY.md` and `.scratch/hydra/build/340a-security-md-known-exposures-and-redactor-classes.md` are modified.
- No em dashes (`—`) appear in `SECURITY.md`.
- `git diff --check` passes.

## Outcome

Run `01a17cd1-f166-77c3-96f9-9ed74aeb7331` on `claude` Haiku 5.5, 3,075,339 tokens.

Changed `SECURITY.md` in the seven places the ticket lists. The `~/.claude.json` sentence, the
codex `WebSearch` and `webrun` lines, the `gate_refusal.reason` marking, the doctor checks, the
`gate_refusal` row rules and the twelve-class export paragraph are in place. The two new known
exposure bullets, `git push` and map 134, are at the end of that section.

Measured: `git diff --check` passes. `SECURITY.md` has no em dashes. Only `SECURITY.md` and this
ticket changed under this ticket. The ticket names no test file and changes only prose, so no
cargo run was needed.

Assumed false at build 361, so the text does not say them:

- Item 5 asked for a sentence that a `gate_refusal` row derives `Outcome::Gated` since build
  ticket 360. Ticket 360 is claimed, not resolved. `refused_everything` in `src/outcome.rs` still
  requires a deny by Hydra's policy and at least one permission decision, so a refusal row alone
  derives `Completed`. The sentence is left out until 360 lands.
- Item 4 asked for "unmeasured" when the installed binary passes `MEASURED_VERSIONS`. `hydra doctor`
  prints the installed and measured versions side by side and prints `unmeasured` only when the
  table has no row for the platform. The text says that.
- Item 7 said `push.default=matching` in a repository config reaches a protected branch because
  the gate does not read repository git config. The gate does read repository config, only
  `branch.<name>.merge` for a bare push. The text says the gate reads only the upstream. It also
  says the gate refuses `--config-env` outright, which the ticket did not say.
