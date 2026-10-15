# 319 - The floor reads a quoted heredoc body as data, single quotes as data, and a newline as a command separator

Type: build
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 120's answer, question 5 and the hole Opus
measured. Two defects in the shell check, fixed in one change because each makes the other
worse alone. First, the substitution rule and the protected-path substring pass read a heredoc
body as if it were the command line: `cat <<'EOF'` whose body mentions `git` is refused, and a
body that spells `~/hydra/runs` is refused, which is what stopped run `01a1332c` (146k tokens,
nothing written) and cost the map 117 GPT seat its memo file. Second, `split_segments` lists
`"\n"` as an operator but `shlex` treats a newline as whitespace and never yields it, so `exit 0`
newline `git gc` passed a live gate while `exit 0; git gc` was refused: a second line starting
with `git push --force` passes the floor on any provider today.

## What

The gate script is compiled by `src/policy/` from `gate.py` (find the source under `src/policy/`
and the compiled copy a run gets under its spawn directory; `tests/gate.rs` compiles a real gate
per policy and feeds it synthetic hook payloads). Change the shell check so that:

1. A newline outside quotes separates commands, the same as `;`. Split on it before `shlex`
   sees the segment, or make the tokenizer yield it; either way `exit 0` newline `git gc` is two
   segments and the second is checked.
2. The body of a heredoc whose delimiter is quoted (`<<'EOF'`, `<<"EOF"`, `<<\EOF`) is data: the
   substitution rule and the protected-path substring pass skip it, from the line after the
   operator to the line that is exactly the delimiter. An unquoted `<<EOF` body expands and stays
   subject to both passes.
3. Text inside single quotes is data for the substitution rule and for the substring pass; double
   quotes are not, since they expand.
4. Everything the floor refuses today outside those three cases is still refused. Attack H3 of
   map 68 (`sh -c 'cat <root>/worktrees/../runs/x.jsonl'`, `tests/gate.rs` line 2008) must stay
   refused: it is a path argument, not quoted data in the sense above, so decide how the
   substring pass tells a quoted path argument from a quoted heredoc body and write the rule
   down in the gate's comments. If the only consistent rule is "single quotes exempt the
   substitution rule but not the protected-path pass", take that and say so in the Outcome.

## Do

1. Read `.scratch/hydra/research/120-seat-opus.md` for the seven probes Opus measured on a live
   gate, and `120-synthesis.md`. Each probe becomes a case in `tests/gate.rs`, the `exit 0`
   newline `git gc` one first, written so it fails before the fix.
2. Fix `gate.py` in the compiler's source, rebuild, run `tests/gate.rs` and the whole suite. The
   corpus under `tests/corpus/` must still refuse every attack it refuses today.
3. Measure on your own live gate (you are inside a run): a `cat <<'EOF'` whose body names git is
   allowed after your change only in your worktree's compiled copy, not in the gate that runs
   you, so measure by running the compiled gate script by hand against a payload, as
   `tests/gate.rs` does.

## Done means

The new cases green, the corpus unchanged in what it refuses, `cargo fmt --all --check`, `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --release --workspace`,
`cargo test --release --workspace --doc`. Amend `IMPLEMENTATION.md` where the pattern floor and
the substitution rule are described, and `docs/harness-spec.md` 7.4 if it states the old rule.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs (the gate that runs you still has the old rule), and
the test suite is green inside a run since build 302 without any `env -u`.
