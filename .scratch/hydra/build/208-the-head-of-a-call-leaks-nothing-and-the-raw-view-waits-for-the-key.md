# 208 - The head of a call leaks nothing, and the raw view waits for the key

Type: build
Status: resolved
Blocked by:

## What

A read-only review by `cursor-agent` (grok-4.7-high) on 2026-10-01 of build 206 (`076e030`). Map
ticket 92 says a default row that shows a secret blocks 206's release, and these do. It found no
panic in `shell_head`, and the ten-second timer fires with no key and no snapshot.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change. The file is
`src/cockpit/call_head.rs` unless an item says otherwise.

1. **A pattern is free text.** `Grep` with pattern `sk-ant-api03-...` shows it. A row shows a
   `file_path`, `path` or `notebook_path` and nothing else; a call with only a pattern reads
   `<tool> ...`.
2. **A subcommand comes from a list, per program.** After a program of `SUBCOMMAND_PROGRAMS` any
   lowercase word shows, so `aws s3cret`, `docker hunter2` and `git hunter2` show it. Give each
   program its own list of real subcommands (`git`: `add`, `commit`, `push`, `pull`, `clone`,
   `status`, `diff`, `log`, `checkout`, `switch`, `branch`, `merge`, `rebase`, `fetch`, `stash`,
   `apply`, `worktree`, `show`, `reset`, `restore`, `tag`, `remote`, `init`; and so on for the
   others, short lists, from each tool's own help). A word not in its program's list is `...`.
3. **A program token that is not one word.** Quotes are stripped and the spaces inside stay, so
   `"echo s3cret"` and `"cat .env"` show whole. A token that held a quote, a space, `$`, a
   backtick, `(`, `)`, `<`, `>`, `{`, `}`, `=` or `!` is not shown: the row is `...`.
   `$(s3cret)` and `$(cat<.env)` are that case.
4. **Flags after `sudo`, `env` and `time`.** `sudo -u root systemctl restart nginx` shows `env ...
   -u ...`, and `env -i cat .env` shows `env ... -i ...`. When the token after the skipped words
   starts with `-`, the row is `env ...` and nothing more.
5. **agy's shell call.** Its frame holds the command under `CommandLine`. Read it as a command
   key, in the row and in the raw view.
6. **The raw view behind `r`** (`tail_line_shown`, and `tail` in `src/cockpit/snapshot.rs` near
   874). It shortens strings under a few keys and copies the rest, so a tool result's `content`,
   `output`, `text`, `aggregated_output`, `streamContent` and `script` are on screen with no
   `i`: the body of `cat .env`. A line over 400 characters is cut before it is parsed and is
   shown raw. Change the default raw line to what cannot carry a payload: `seq`, `ts`, the
   record's `type` or the provider frame's `type`, the tool's name when there is one, the head
   when it is a call, and the line's length. `i` on the selected line shows it as it is, cut to
   the pane. Parse before cutting. A line that is not JSON shows as `not JSON, <n> bytes`.
7. **The reveal is bound to what was selected.** `i` keeps a row index, and a new snapshot slides
   the last 15 calls and the last 20 raw lines under it, so the pane shows another call's input.
   Keep the text that was revealed and show that until it closes.
8. **Three ways out that leave it open** (`src/cockpit/host.rs` near 113 to 142). `F2`, `Ctrl-q`
   and a click on the chat return before `App::key`. Each closes the reveal.

Known and left as it is, say so in `IMPLEMENTATION.md`: the program's own base name shows, so
`./deploy-s3cret` shows `deploy-s3cret`. A program name cannot be told from a secret by its
shape.

`IMPLEMENTATION.md`: the cockpit passage on the call row and the raw view.

## Done when

- Each test failed before the change; fmt, clippy over the workspace, `cargo test --lib` and
  `cargo test --test cockpit` pass.

## Outcome

Resolved 2026-10-01. Two runs: `01a0f650` (codex gpt-6.1-sol) wrote the tests and most of
`call_head.rs` and stopped at the account's usage limit; the session applied its unfinished diff
to master uncommitted, and `01a0f659-191b` (cursor-agent composer-2.5, 0.7M tokens) finished it
from there. Diff applied to master as written.

Built: a row shows a path and never a pattern; each program of the list has its own subcommands
and any other second word is `...`; a program token that held a quote, a space, `$`, a backtick,
a bracket, `<`, `>`, `=` or `!` is `...`; a flag after `sudo`, `env` or `time` leaves the row at
`env ...`; agy's `CommandLine` is a command key; the raw view shows `seq`, `ts`, the type, the
tool, the head and the line's length, parses before it cuts, and says `not JSON, <n> bytes` for
the rest; `i` keeps the text it revealed, so a snapshot that slides the list does not change
what is shown; `F2`, `Ctrl-q` and a click on the chat close the reveal.

Measured on master: fmt and clippy clean on the cockpit's files, `cargo test --test cockpit` 109
passed, `cargo test --lib` 224 passed. Neither run saw the eight `ticket208_*` tests fail before
the change; the session reverted `src/cockpit/` with the tests in place and all eight failed,
then restored it.

Known and left: the program's own base name shows, so `./deploy-s3cret` shows `deploy-s3cret`.

