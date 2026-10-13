# Map 120, research seat memo (Gemini 3.8 Flash)

Written 2026-10-13 by one seat of the four-seat panel on map ticket 120. My mandate is to answer the five design questions on the pattern floor and worktree path scope based strictly on what is implemented, measured, and verified in this worktree. Citations reference relative paths within this repository. No provider was launched.

---

## 1. Carving out the run's own worktree path

### What the code does today
The gate script evaluates shell commands through two distinct layers:
1. The protected path check in `src/policy/gate.py:1124-1143` runs `protected_path_named_by(raw_command, cwd)` (`src/policy/gate.py:283-352`). This performs a raw substring match against `denied_prefixes()` (`src/policy/gate.py:335-339`), followed by a token pass checking `protected_root(resolve(candidate, cwd))` (`src/policy/gate.py:346-352`).
2. In `src/policy/gate.py:271-280`, `protected_root(path)` checks `protected_worktree_paths` (denying hook and MCP configurations in the worktree), then checks `own_write_paths` (`src/policy/gate.py:274-276`: `if inside(path, own): return None`), and finally checks `deny_write_paths` (`src/policy/gate.py:277-279`).
3. During gate compilation in `src/policy/mod.rs:964-1004`, `denied_spellings` filters out any denied path where `holds_own` is true (`src/policy/mod.rs:983-986`). Because `spawn.worktree` resides under `layout.worktrees()`, `layout.worktrees()` is omitted from `denied_spellings`, preventing the raw substring pass from blanket-refusing the worktree parent substring.
4. However, if a command names the parent directory `~/hydra/worktrees` directly, uses `..` from the worktree root, or uses an unexpanded glob like `/Users/USER/hydra/worktrees/*`, the candidate does not fall inside `own_write_paths`. `protected_root` therefore falls through to `deny_write_paths` (matching `layout.worktrees()` or `hydra_home()`), returning a refusal under `hydra/protected-path` (`src/policy/gate.py:1138-1143`).
5. Separately, the pattern floor in `src/policy/gate.py:431-624` enforces argument conditions on git, aws, and Slack. In `src/policy/gate.py:592-600`, any command containing substitution markers (`$(`, `${`, `` ` ``) that also contains `git`, `aws`, or `slack.com` is refused outright as `"a shell substitution in a command naming git, aws or Slack"`. If a script or provider prints `pwd` inside a command substitution such as `git status $(pwd)`, the pattern floor refuses it under `hydra/floor`.

### What was measured and verified
In this worktree (`/Users/USER/hydra/worktrees/01a13736-9575-7a00-a5fb-a92b40f8747b`), I executed:
- `rg "pattern floor" /Users/USER/hydra/worktrees/01a13736-9575-7a00-a5fb-a92b40f8747b/CLAUDE.md`: allowed (exit 0).
- `git status /Users/USER/hydra/worktrees/01a13736-9575-7a00-a5fb-a92b40f8747b`: allowed (exit 0).
- `git -C /Users/USER/hydra/worktrees/01a13736-9575-7a00-a5fb-a92b40f8747b status`: allowed (exit 0).
- `tests/gate.rs:2589`: `format!("rg TODO {worktree}")` is explicitly tested in the test suite and passes (`test the_raw_pass_leaves_an_ordinary_command_alone`).
- `ls /Users/USER/hydra/worktrees`: denied with `hydra/protected-path: the command names /users/user/hydra, denied under every policy by harness spec 7.5.`

The token pass already permits exact paths inside the run's own worktree via `protected_root`. The failure mode occurs when providers paste or construct ancestor paths (`~/hydra/worktrees`), globs, or command substitutions combining `pwd` with git invocations.

### Recommendation
Carve out the run's own worktree (`cwd` from `run_start` with a trailing path separator) before path and substitution matching in `protected_path_named_by` and `floor_violation`, while keeping sibling worktrees, `runs/`, and `spawn/` strictly denied. This maintains isolation between concurrent runs while allowing agents to reference files by their canonical absolute path without accidental gate refusals.

### What would change my mind
Evidence that allowing a run to name its own canonical worktree path permits sandbox escapes via relative traversal past `cwd/..`, or proof that the kernel seatbelt fails to block sibling directory access when the shell gate permits the worktree prefix.

---

## 2. Refusal message diagnostics

### What the code does today
When a shell command names a protected path, `src/policy/gate.py:1138-1143` emits:
`hydra/protected-path: the command names <path>, denied under every policy by harness spec 7.5. A shell command carries no path for the gate to check, so one that names a protected path is refused whole.`

When the pattern floor catches a command, `src/policy/gate.py:1213-1216` emits:
`hydra/floor: the pattern floor refuses <hit>. Lift it for one run with --allow-floor-escape.`

Neither error message identifies whether the refused string was an ancestor or sibling of the run's own working directory, nor do they suggest using relative paths. Because `--allow-floor-escape` is a human-only launch flag (`docs/harness-spec.md` section 16), an autonomous provider cannot lift the gate and instead spends context trying alternative shell incantations.

### What was measured and verified
Running `ls /Users/USER/hydra/worktrees` produced:
`hydra/protected-path: the command names /users/user/hydra, denied under every policy by harness spec 7.5. A shell command carries no path for the gate to check, so one that names a protected path is refused whole.`

In run `01a1332c` (cited in ticket 120), the agent consumed 146,000 tokens in repetitive attempts after receiving generic refusals, writing nothing. In contrast, map ticket 96 established the architectural principle of providing actionable refusals that name the way out ("refusals that name the way out").

### Recommendation
If an absolute path matches a protected ancestor or sibling directory, the refusal should explicitly diagnose the cause: `"hydra/worktree-path: your command names absolute path under protected directory {}; use a relative path instead."` Providing this immediate actionable feedback enables language model agents to correct their command in a single turn rather than burning quota thrashing against a policy boundary.

### What would change my mind
Empirical evidence that models fail to follow relative-path redirection prompts, or evidence that disclosing path hierarchy details introduces an information leak that enables adversarial path traversal.

---

## 3. macOS seatbelt profile interaction

### What the code does today
In `src/sandbox.rs:111-116`, `compose` writes denial rules for every entry in `write_denies(set, spawn)`. This includes `layout.worktrees()` (`~/hydra/worktrees`).
Subsequently, in `src/sandbox.rs:207-215`, `compose` iterates over `own_writes` (`policy::own_write_paths(spawn)`):
```rust
for path in own_writes {
    let own = canonical(&path);
    lines.push(format!(
        r#"(allow file-write* (subpath "{}"))"#,
        escape(&own)
    ));
}
```
As documented in `src/sandbox.rs:427-432`, macOS Seatbelt (SBPL) follows a last-matching-rule semantic:
`"SBPL takes the last matching rule, measured 2026-09-21: a deny on worktrees/ followed by an allow on worktrees/<own> leaves the own tree writable and a sibling refused."`

At the OS kernel level, writes under the run's own worktree via absolute paths are already explicitly permitted. The refusal observed during tool execution is enforced entirely in user space by `src/policy/gate.py`.

### What was measured and verified
Inspecting `src/sandbox.rs:207-215` confirms that `(allow file-write* (subpath "<own_worktree>"))` is already generated in every recorded macOS launch profile.
In this worktree, file writes via standard tools succeed under the seatbelt. The disagreement identified in ticket 120 is between `gate.py`'s user-space string rejection and `sandbox.rs`'s kernel-level allow rule.

### Recommendation
No changes to the seatbelt profile in `src/sandbox.rs` are required because the profile already carves out the run's own worktree by absolute path. Aligning the gate script's shell checks with `sandbox.rs` eliminates an unnecessary discrepancy where the gate refuses strings that the kernel sandbox was intentionally configured to permit.

### What would change my mind
Evidence that macOS `sandbox-exec` fails to resolve symlinks or canonize subpaths consistently when handling nested subpath allows inside denied parents, or if a new read-carveout requirement in `sandbox.rs` exposed sibling worktrees to kernel read operations.

---

## 4. Cerebro repository placement vs. Hydra self-edit

### What the code does today
1. `src/run/mod.rs:452-460` (`phase_worktree`) constructs the run's worktree using:
   `machine.reserved.layout.worktree(&machine.reserved.run_id)`
2. `src/paths.rs:144-149` defines `layout.worktrees()` as `self.root.join("worktrees")`, which is always `<HYDRA_ROOT>/worktrees` (`~/hydra/worktrees/`).
3. Regardless of whether the seed repository is Hydra itself (`~/hydra`) or an external repository (`~/cerebro`), the worktree destination is invariably created under `~/hydra/worktrees/<run_id>/`.
4. In `src/policy/mod.rs:794-853` (`denied_write_paths`), `layout.worktrees()` is added to `paths` for all runs, irrespective of the target repository.
5. In `src/policy/mod.rs:843-845`, `spawn.parent_git_dir` is added to `denied_write_paths`. For Hydra self-edit runs, this points to `~/hydra/.git`; for Cerebro runs, it points to `~/cerebro/.git`. In both cases, the run's worktree path sits under `~/hydra/worktrees/`.

### What was measured and verified
1. Repository worktree paths:
   - Current Hydra self-edit worktree: `/Users/USER/hydra/worktrees/01a13736-9575-7a00-a5fb-a92b40f8747b`
   - Cerebro run `01a1332c` worktree: `/Users/USER/hydra/worktrees/01a1332c-...`
   Both are located under `~/hydra/worktrees/`.
2. Attack corpus audit (`tests/corpus/README.md`):
   - `tests/corpus/README.md` documents exactly 14 attack classes (A1 through A14).
   - Exactly 0 of the 14 recorded attack classes (0%) relied on an agent naming its own absolute worktree path.
   - Attacks in A6, A9, and A10 specifically probed sibling worktree access (`worktrees/other` in `tests/gate.rs:400, 508`), parent directory laundering (`worktrees/../runs/` in `tests/gate.rs:2008`), and symlink escapes (`tests/gate.rs:758`). None targeted or required blocking the run's own worktree path.

The Cerebro case is therefore not special. Every repository launched under Hydra operates within `~/hydra/worktrees/<run_id>/` and encounters the exact same path and floor rules.

### Recommendation
The Cerebro case is not special because all runs across all repositories place their worktrees under `~/hydra/worktrees/` (`src/run/mod.rs:456`), subjecting them to identical gate checks. An audit of `tests/corpus/README.md` confirms that 0 of 14 attack classes relied on an absolute worktree path, with attacks exclusively targeting sibling paths (`worktrees/other`) and parent traversal (`worktrees/..`).

### What would change my mind
Evidence that external repositories with non-standard git worktree configurations (such as linked common dirs or external alternates) resolve `own_write_paths` to a path that bypasses `holds_own`, or evidence of an attack vector where naming the own worktree path allows spoofing git object provenance.

---

## 5. Heredoc and substitution handling in the pattern floor

### What the code does today
1. `src/policy/gate.py:435` defines `FLOOR_TARGETS = ("git", "aws", "slack.com")`.
2. `src/policy/gate.py:438-440` and `policies/floor.json:7` define substitution markers: `["$(", "${", "`"]`.
3. In `src/policy/gate.py:448-461`, `split_segments(command)` parses commands with `shlex.shlex` and splits on operators:
   `operators = {";", "&&", "||", "|", "&", "\n", "(", ")"}`
   Crucially, newline (`\n`) is treated as a command delimiter.
4. In `src/policy/gate.py:592-600`:
   ```python
   if has_substitution(command):
       lowered = fold_ascii(command)
       if any(target in lowered for target in FLOOR_TARGETS):
           return "a shell substitution in a command naming git, aws or Slack"
       return None
   ```
5. If a shell command contains a heredoc (e.g. `cat <<'EOF'` or `python3 - <<'PY'`), two separate failure modes occur:
   - If the heredoc body contains any newline followed by a word starting with `git` (such as `git status` or explanatory text like `git is a VCS`), `split_segments` treats that line as a distinct command segment where `argv[0] == "git"`, evaluating it against `check_git(argv)` (`src/policy/gate.py:614-616`).
   - If the heredoc body contains any backtick, `${VAR}`, or `$(cmd)` expression (common in markdown spans, shell snippets, or python scripts) while mentioning `git`, `aws`, or `slack.com` anywhere in the body, `has_substitution(command)` evaluates to true and rejects the command under `hydra/floor`.

### What was measured and verified
1. Build 238 first attempt (`.scratch/hydra/build/238-local-llm-in-rust-the-cutover.md:27-30`):
   Stopped when the floor refused a `python3 - <<'PY'` script whose code mentioned git.
2. Map 117 panel memo (`.scratch/hydra/issues/120-the-floor-and-the-runs-own-worktree-path.md:39-41`):
   Cost the GPT seat its memo file on disk when a `cat <<'EOF'` containing markdown backticks and mentioning git was refused by the floor.
3. In shell syntax (POSIX.1-2017), a quoted heredoc (`<<'EOF'`, `<<"EOF"`, or `<<\EOF`) disables all parameter expansion, command substitution, and arithmetic expansion within the heredoc body. The body is strictly stdin data for the invoking command and cannot execute shell subcommands.
4. Under Hydra's core security contract (`docs/harness-spec.md` section 8), input fed to tools or stdin streams is data, not instructions. Conflating stdin heredoc bodies with executable argv segments is a false positive of the parser.

### Recommendation
The gate should distinguish the executable command line from stdin data by stripping quoted heredoc bodies (`<<'EOF'`) before evaluating `has_substitution` and `split_segments`, because quoted heredocs cannot execute code and their bodies represent data rather than instructions. Unquoted heredocs containing command substitutions (`$()`, `` ` ``) should continue to be refused, preserving the floor's protection against indirect execution while eliminating false-positive blocks on file writes and inline script execution.

### What would change my mind
Evidence that an adversary could exploit shell parser discrepancies between Python's `shlex` and `/bin/sh` or `/bin/zsh` to conceal an executable command inside an ostensibly quoted heredoc marker, or evidence that stripping heredoc content introduces catastrophic regex backtracking in `gate.py`.

---

## Recommendation summary
1. Carve out the run's own worktree path (`cwd` with trailing separator boundary) before path checks, while keeping sibling worktrees, `runs/`, and `spawn/` strictly denied.
2. If an absolute worktree path is refused, update the message to state `"your command names an absolute path under protected directory {}; use a relative path instead"` so agents recover in one turn.
3. Keep the macOS seatbelt profile in `src/sandbox.rs` unchanged, as it already permits writes to the run's own worktree by absolute path.
4. Treat the Cerebro case as standard rather than special, as all runs across all repositories place their worktrees under `~/hydra/worktrees/` and 0 of 14 attack classes in `tests/corpus/` relied on an absolute worktree path.
5. Distinguish commands from stdin data by stripping quoted heredoc bodies (`<<'EOF'`) prior to running substitution and segment checks, while continuing to refuse unquoted heredocs containing command substitutions.
