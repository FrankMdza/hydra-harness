# 206 - The detail shows the head of a call, and a key shows the rest

Type: build
Status: open
Blocked by: 205

## What

Map ticket 92 (2026-10-01, read its `## Answer`): the cockpit's pattern mask for secrets
(`masked` and `secret_at` in `src/cockpit/snapshot.rs`, builds 180, 200, 201, 203) is replaced.
The detail's row for a tool call shows only the head of the call. A key on the selected call
shows the whole input, unmasked, and it leaves by itself.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change.

1. **The head of a shell call.** The program's base name and at most one more word, kept only
   when that word is a bare subcommand: lower-case ASCII letters, digits and `-`, not starting
   with `-`, with no `=`, `/`, `.`, `:`, `@`, quote or `$`. So `curl`, `git push`, `cargo test`,
   `bash`, `python3`, `aws configure`, `docker run`. A leading `NAME=value` assignment, `sudo`,
   `env` and `time` are skipped to reach the program, and the row says `env ... program`. A
   pipeline or `&&` list shows the head of its first command and `...`. When anything was left
   out the row ends in ` ...`; when nothing was, it does not.
2. **Other tools.** The tool's name and its path or pattern argument (`Read src/x.rs`, `Grep
   foo`), cut to the row. A path is shown as it is. An MCP or unknown tool shows its name only.
3. **The reveal.** A key on the selected call (choose one that is free and name it in the help
   line) replaces the row with the whole input, wrapped, unmasked. The next key of any kind
   closes it, and it closes by itself after ten seconds (`REVEAL_SECONDS`, one constant). A
   snapshot that arrives does not close it and does not extend it. A call on a record line with
   `secret_flag` stays `input hidden: secret_flag`, and the key does nothing there but say so.
4. **The raw view behind `r`** shows record lines, input included. It now opens with the same
   rule: each line's tool input shown as its head, and the reveal key shows the selected line.
5. **Delete the pattern mask**: `masked`, `secret_at`, `secret_value_end`, `token_end`,
   `enclosing_quote`, `quoted_end`, `SECRET_NAMES`, and the tests that pin their patterns.
   `record::secrets::prefixes` stays, the sweep uses it.
6. **The test that is the release condition.** Every input the five reviews listed, taken from
   the tickets 180, 200, 201, 203 and from map ticket 92's question, goes through the default
   row, and none shows its secret: `curl -u user:s3cret`, `mysql -ps3cret`, `sshpass -p x`,
   `curl -H 'Cookie: sid=s3cret'`, `curl -H Authorization:"Bearer s3cret"`, `curl -d
   '{"Authorization":"Bearer s3cret"}'`, `aws configure set aws_secret_access_key s3cret`,
   `curl -k s3cret`, `https://user:s3cret@host`, `TOKEN=abc ./deploy --prod`, `bash -c
   "TOKEN=abc ./deploy --prod"`, and the rest. Each head still names the program.

`IMPLEMENTATION.md`: the cockpit passage on the detail's masking, rewritten for this rule.
`README.md` if it names the mask.

## Done when

- Each test failed before the change; fmt, clippy over the workspace, `cargo test --lib` and
  `cargo test --test cockpit` pass. The report lists, for twenty real commands of your choosing,
  the head the row shows.
