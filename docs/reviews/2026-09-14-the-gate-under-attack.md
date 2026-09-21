# The gate under attack

Date: 2026-09-14. Method: an agent was told to find a way out and to prove each one. It drove
the real compiled gate of a live run with synthetic hook payloads, launched no provider, and
was told that the shell-interpretation gap of spec 7.4, the absence of a container and Cursor's
three unhooked tools are documented limits rather than findings. It was also told to report the
attacks it expected to work and that did not, because that half is evidence too.

Twelve attempts. Five holes, three of them verified by hand against the same gate before
anything was changed.

**What this report does not print.** Four of the five are still open, and this file gives their
shape, their ticket and their severity, and not a probe anyone could run. That is the rule in
`README.md` beside this file, decided before the review started. The fifth is fixed and its
probe is a test in the repository.

## Fixed

**The shell check only saw whole tokens.** Spec 7.5 says a command that names a denied path is
refused whole, and `cat /Users/USER/hydra/runs/x.jsonl` was refused. The same path inside a
larger token was not: an interpreter's `-c` argument is one token to `shlex`, so the corpus,
the skill store, the snapshots, a sibling's spawn directory and the parent repository's `.git`
were all reachable. This was not the documented limit. That one is a path the command never
writes down, assembled from a variable; here the path was written down in full and the parser
missed it.

`protected_path_named_by` now runs a raw pass before tokenizing: the command, case-folded, is
searched for each denied path as a substring, in both spellings macOS allows, since
`/var/folders/...` and the `/private/var/folders/...` it resolves to are both things a command
can write down. Two tests in `tests/gate.rs` pin five shapes of the bypass and five ordinary
commands that must keep working.

**The residual, stated rather than hidden.** A denied path with one of the run's own paths
under it cannot go on the raw list: `~/hydra/worktrees` is denied while the run's own worktree
sits inside it, and a substring search has no way to say "this one and not that one". So a
sibling run's worktree, spelled inside a larger token, still reaches the token pass and still
walks through. The corpus, the store, the snapshots, the spawn directory and the per-run config
files carry no own path and are covered. `gate.py` says this where the code is.

## Open

| Finding | Severity | Ticket |
|---|---|---|
| A run can relaunch the binary and name its own policy. Spec 7.3's rule that a child is never wider than its parent is not enforced anywhere: nothing marks a process as being inside a run, and `standard` grants the shell that reaches the binary | critical, fixed 2026-09-15: `HYDRA_RUN_ID` marks every gated child and a launch carrying it without the matching `--parent` is refused; a child that unsets it walks through, and `SECURITY.md` says so | build 57 |
| The floor's `git` parser consumed the wrong token for one flag, so a refused command passed when that flag was present. Fixed 2026-09-21: `git -c key=value` and `git --config-env key=VAR` take a separate value the way `-C` does, and the parser read that value as the subcommand, so `git -c core.pager=cat push --force origin main` walked past the push rules. Two entries in `consume_before_subcommand` in `policies/floor.json`, made by the human because it is protected config #1, and `tests/gate.rs` holds the four commands | real, fixed | build 58 |
| Two providers offer a shell tool that carries no command string, so neither the floor nor the path check has anything to read | real, fixed 2026-09-15: both rows carry a capability no policy grants | build 59 |
| Under `permissive`, an MCP tool's path never meets the deny list, because `evaluate` applies it to `file_write` alone. No admitted server writes today, so this is a loaded gun rather than a fired one | narrow, fixed 2026-09-15: the deny list meets MCP paths under every policy | build 59 |
| The compiler case-folds with Rust's `to_lowercase` and the gate with Python's `casefold`. They disagree on characters like `ß`, so a non-ASCII home or repository name silently drops every denial and every scope | theoretical, total when it fires; fixed 2026-09-15: both sides fold ASCII only | build 60 |

One more, which is not a bug and is the honest frame for all of the above: **the deny list
covers `~/hydra`'s runtime directories and five config files, and nothing else.** Under
`standard`, shell reads and writes the rest of the machine, including `~/.ssh`, the user's own
`~/.claude` guards, Hydra's own source, and any directory that precedes `/usr/bin` on the
human's PATH. `CLAUDE.md` claims a read of `~/.claude/` is refused under `standard`; that is
true of the `Read` tool and false of `cat`.

## Three attacks that did not work

**A symlink out of the worktree.** `resolve` calls `realpath` before matching, so a link inside
the worktree pointing at the corpus was refused as a protected path, a link to `/etc` was
refused as out of scope, `..` traversal was refused, and an uppercase spelling of the corpus
was still caught. A hard link beats it, and reaching one needs the fixed finding above.

**Emptying or replacing the gate script.** The hook command wraps the script in a sha check the
provider snapshots at process start. Emptied, replaced with an unconditional allow, and
deleted: all three answered `hydra/gate-tampered` and exited 0. The historical fail-open on
`claude`, an empty script exiting 0 with no output, is genuinely closed.

**Making the gate crash or shrug.** An unknown tool name, another provider's tool name, the
wrong dialect, no dialect, malformed JSON, an empty payload, a null cwd and a non-string path
all deny. With no dialect on argv the script prints both refusal shapes and exits 2, so an
`agy` hook fails closed on the exit code alone.

## What the reviewer said would help most

Stop enforcing containment by reading argv. Every finding above except the case-folding one is
the same bug in a different spelling: the gate is a string matcher in front of a shell that can
write a path a thousand ways, and `standard` hands that shell to the model by design. Run
children under a different uid, with the corpus, the store, the spawn directory and the policy
directory not writable by it, and `sh -c`, a hard link, a variable and a ligature all fail on
the same `EACCES`. The gate goes back to what it is good at, which is shaping capabilities and
writing down what was asked. That is map ticket 62, and it is the largest open question in this
project's security model.

## What this review changed beside the code

The corpus walk went red the moment the shell check got stricter, because a record from
2026-09-08 carried a command the gate of that day allowed. That is the third time a change to
the gate's own code turned every older record into a leak. Build ticket 49 became the work of
the same evening: `run_start` carries `gate_sha` now, and a record that does not name the gate
that answered for it gets a limit where it used to get a leak, because the derivation cannot
tell a leak from a gate whose code has changed since.
