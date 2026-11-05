# 203 - The mask covers URLs and `Authorization`, and `y` checks the goal it runs

Type: build
Status: resolved
Blocked by:

## What

A read-only review by `cursor-agent` (grok-4.7-high) on 2026-10-01 of build 201 (`1c88055`). It
confirmed the rest of 201 by probe: quoted flag values, `?api_key=...&x=1`, a tab after a colon,
trailing escapes, nested `$(...)`, and `nonfinite_json` against `json.loads` on a 20,000-line
fuzz. Six defects are left, and one rule hides more than it should.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change.

1. **A URL password and an `Authorization` header** (`src/cockpit/snapshot.rs` near 906).
   `https://user:s3cret@host` and `--header "Authorization: Bearer abc"`, also `Authorization:
   Basic dXNlcjpwYXNz`, are drawn unchanged. Mask the password of `scheme://user:password@`, and
   the whole value of an `Authorization` or `Proxy-Authorization` header, scheme word included or
   not, and say which.
2. **A one-dash flag with a secret name** (near 908). `-api-key s3cret`, `-password s3cret` and
   `-key s3cret` show the value; only `--` takes a value after a space. One dash does too.
   `-k s3cret` stays open: no name to match.
3. **A bare value that starts with a backslash** (near 971). `TOKEN=\s3cret` is drawn as
   `TOKEN=***\s3cret`, because `token_end` stops at `\` and the mask covers nothing. A backslash
   escapes the next character inside a bare value.
4. **A colon value with a space, inside quotes** (near 965 and 971). `curl -H 'x-api-key: s3cret
   more'` is drawn as `'x-api-key: *** more'`: the quote opened before the name, so the scan stops
   at the space. When the name sits inside a quoted word, the value runs to that closing quote.
5. **The chat host's status line at heights 1 to 3** (`src/cockpit/host.rs` near 271).
   `App::draw_status` returns on an empty area since build 201; `Host::draw_status` does not, and
   with a session it panics on the buffer's bottom edge. Return on an empty area. Test heights 1
   to 5 at widths 1 to 20 with a session, and by running the draw, not by reading it.
6. **`y` checks the goal on screen and runs the goal in the confirmation** (`src/cockpit/app.rs`
   near 1012 and 1305, `set_snapshot` near 323). With no `--goal` the cockpit follows the log
   that changed last. Open a confirmation on goal A, let goal B become the shown goal, damage A:
   `y` reads B, finds it healthy, and runs approve or review on A. Check the goal the command
   names: fold that goal's log at `y`, or drop an open confirmation when the shown goal changes,
   and say which and why.
7. **A flag's value that is itself a flag.** `log --pass-through --oneline` becomes `log
   --pass-through ***`. After a space, a word that starts with `-` is not a value and is left
   alone. `--key-file path`, `--keyboard us` and `key: enabled` stay masked: hiding a path is the
   cheaper mistake.

`IMPLEMENTATION.md`: the cockpit passage on the detail's masking, and the one on the damaged goal.

## Done when

- Each test failed before the change; fmt, clippy over the workspace, `cargo test --lib` and
  `cargo test --test cockpit` pass.

## Outcome

Resolved 2026-10-01. Run `01a0f610` (codex gpt-6.1-sol, `standard`, 1.6M tokens), diff read whole
by the session and applied to master as written.

Built: the password of `scheme://user:password@` is masked; the whole value of `Authorization`
and `Proxy-Authorization`, scheme word included; a one-dash flag with a secret name takes a value
after a space; a backslash escapes the next character in a bare value; a secret name inside a
quoted word masks to that closing quote; after a flag and a space, a word that starts with `-` is
left alone; `Host::draw_status` returns on an empty area. For item 6 the run chose to drop an open
step or review confirmation when the shown goal changes, with a message, so `y` can only ever act
on the goal on screen; build 201's check at `y` covers the same goal turning damaged.

Measured on master: fmt and clippy clean over the workspace, `cargo test --test cockpit` 101
passed, `cargo test --lib` 224 passed. The session reverted `snapshot.rs`, `app.rs` and `host.rs`
and saw all nine new tests fail, then restored them.

Left open: a secret name inside a quoted word hides the rest of that word, so `bash -c
"TOKEN=abc ./deploy"` shows `TOKEN=***"` and not `./deploy`. `-k value` still shows its value.
`enclosing_quote` scans from the start of the row for every match, which is quadratic on a row
with many matches.

