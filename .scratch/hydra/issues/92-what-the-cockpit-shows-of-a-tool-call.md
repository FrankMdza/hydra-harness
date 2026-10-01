# 92 - What the cockpit shows of a tool call's input

Type: task
Status: resolved
Blocked by:
Opened: 2026-10-01, by the session, after the fifth review of the cockpit's secret mask.

## Question

The cockpit's detail pane shows one row per tool call of a run: the tool and a one-line summary
of its input, for a shell call the command. Build 180 added a mask that turns secrets in that row
into `***`. It is a list of patterns in `src/cockpit/snapshot.rs`: token prefixes from
`src/record/secrets.rs`, and the value after a name that holds `TOKEN`, `KEY`, `SECRET`,
`PASSWORD` or `PASS`. Builds 200, 201 and 203 each widened it after a review, and each review
since has found more:

- Still shown after build 203 (codex review, 2026-10-01): `curl -u user:s3cret`, `mysql
  -ps3cret`, `sshpass -p x`, `Cookie: sid=...`, `Authorization:"Bearer x"`, a JSON
  `"Authorization"`, `machine host login user password s3cret`, `aws configure set
  aws_secret_access_key s3cret`, and `-k s3cret`.
- Hidden that a human needs, since build 203: `bash -c "TOKEN=abc ./deploy --prod"` shows `bash
  -c "TOKEN=***"`, so the deploy and its target are gone. Also `--key-file path`, `key: enabled`.

The row exists so the human can judge what a run is doing, and approve or stop it. The record on
disk is not masked here; `secret_flag` on a record line already hides its input entirely. Nothing
in this question changes what a run can do or what the record holds.

Options:

- A. Keep the pattern list and keep fixing it, one build per review.
- B. Show the row unmasked. The cockpit runs in the human's own terminal, on their own machine,
  reading their own records; drop the mask and say so.
- C. By default show only the head of a shell command: the program and its words up to the
  first flag, assignment, quote or path-like argument (`curl`, `git push`, `bash -c`), with `...`
  for the rest. A key on the selected call shows the whole input unmasked for as long as it is
  held open. Other tools show the tool name and the path or pattern argument only.
- D. As C for the default row, and the key shows the whole input through the pattern mask of
  today, frozen as it is.

## Answer

Resolved 2026-10-01 by a panel of three under the human's delegation. Unanimous: **C**. codex
0.76, cursor-agent grok-4.7-high 0.71, agy gemini-3.1-pro-high 0.90; all three named D second.
Answers in the human's decision log, `decisiones/2026-10-01/answer-92-*.md`.

The row shows the head of the call and nothing else by default, and a key on the selected call
shows the whole input unmasked. The pattern mask goes: five reviews showed it has no finish
line, it leaks on a shared screen and hides the deploy target at once, and a head that is too
short is seen at a glance where a missed pattern is not.

Conditions the seats attached, which build 206 carries:

- What is uncertain stays hidden (codex). The head is the program and a bare subcommand; when a
  review cannot agree where a path-like argument starts, the stricter head ships (cursor).
- A cut row is told apart from a command that simply ended there (agy): the row ends in `...`
  only when something was left out.
- The whole input is on screen only on request and leaves by itself (cursor, agy). All three
  wrote "while the key is held". A terminal does not report a key's release without the kitty
  keyboard protocol, so the session set it as: the key opens the full input, the next key of any
  kind closes it, and it closes by itself after ten seconds. The human can change that number.

What reverses it, from the seats: the reveal used on more than half of the tool calls, or open
longer than ten seconds on more than half of real approvals (then D, the frozen mask behind the
key); a default row that leaks a secret blocks the release of 206.

A record line with `secret_flag` stays `input hidden: secret_flag`, and the reveal does not open
it. Carried by build 206.

