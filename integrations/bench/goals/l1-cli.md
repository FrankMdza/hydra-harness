# l1-cli

Size: large

Turn `tally` into a small CLI as well as a library.

- A binary named `tally` that reads stdin, or a file named on the command line, and prints
  three lines: `words`, `lines`, `chars` (add `chars` if it is not already there: non-whitespace
  characters).
- `--json` prints one JSON object with those three keys instead of the three lines.
- Tests for stdin, a file argument, `--json`, and an empty input.
- A README section that shows both output shapes.

Do not rewrite the library functions that already exist. Done means `cargo test` is green,
`cargo run --quiet -- --json` on an empty stdin prints `{"words":0,"lines":0,"chars":0}`, and
the README documents both shapes. This is several tasks; plan them, then assign them.
