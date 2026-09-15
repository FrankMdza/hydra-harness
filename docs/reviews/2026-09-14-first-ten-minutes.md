# First ten minutes: a cold clone follows the README

Date: 2026-09-14. Method: an agent that had never seen this repository cloned it to `/tmp`,
followed the README's install and quick start exactly as written, launched one `agy` run under
a scratch `HYDRA_ROOT`, and read the docs as a newcomer. It was told to be blunt and to record
where it had to guess. Nothing in this report is held back: every defect it found is either
fixed below or has a ticket.

## What worked

`cargo build --release` in 18.9 seconds with no warnings. `--help` on every command. One run
end to end, recorded and reaped:

```
$ HYDRA_ROOT=/tmp/hydra-scratch-root hydra run --provider agy --policy read-only \
    --timeout 3m --task "Reply with the single word OK and stop."
hydra: run 01a0a07c-6517-7373-97c8-f876965b7a5b
"outcome": "completed", "outcome_source": "derived", "exit_code": 0
```

Argument errors read well: `Error: nope is not one of the five policies: judge, curate,
read-only, standard, permissive`.

## The finding that cost more than the rest combined

**A fresh clone reads and writes the corpus at `~/hydra`, whatever directory the checkout is
in.** The first command in the quick start printed 197 runs the reviewer had not made.
`Layout::discover()` reads the passwd home and appends `hydra`. A run launched from that clone
would have written its record, worktree and skill snapshot into somebody else's root.

`HYDRA_ROOT` moves everything and worked, and it appeared in no user-facing document: one doc
comment in `src/paths.rs` saying "it exists for one test file", and one line of CI.

In the reviewer's own words, asked whether it wanted the tool after five minutes: it could say
what Hydra is for, and could not answer that, "because I could not tell whether running it
would disturb anything of mine."

Fixed the same day: `hydra status` prints `root <path>` before any count, and the README says
where Hydra writes, that the binary is not relocatable away from its checkout, and how
`HYDRA_ROOT` moves it. The deeper question, whether a checkout should be separable from a data
root at all, is map ticket 61.

## Fixed the same day

| # | Finding | Fix |
|---|---|---|
| 1 | `status` never said which root it read | prints `root <path>` first, pinned by a test |
| 2 | `HYDRA_ROOT` undocumented | README's install section |
| 3 | `hydra logs` rejected the prefix `show` and `reap` accept, two lines after the quick start teaches it | `logs` resolves a prefix |
| 4 | `SECURITY.md` described build ticket 44's hole as open, four days after it closed | rewritten to say what is true |
| 5 | The README's pre-flight table promised an auth check that is specified and not built | the table says what the code checks and what it does not |
| 6 | `hydra export` refused with no way to learn the format, and no example file exists | the error names the shape, `config/export-allowlist.json.example` ships |
| 7 | A runtime message named `agy` 1.1.26 to a reader on 1.2.2 | says the version it was measured on |
| 8 | The install line had no URL and no toolchain version | both |
| 9 | `usage` in the command table without its subcommand | `usage recount` |

## Open, with tickets

| Finding | Ticket |
|---|---|
| `policies/` and `config/` resolve through `CARGO_MANIFEST_DIR`, so the binary is not relocatable and nothing says so at runtime | build 50 |
| `tests/record_contract.rs` reads the real home and ignores `HYDRA_ROOT`, so a contributor's suite validates their own corpus whatever they set, and CI's comment claims otherwise | build 51 |
| `docs/demo/record.sh` and `verdict.py` hardcode `~/hydra` and do not work from a clone | build 52 |
| Whether a checkout and a data root should be separable, and what `hydra doctor` would report | map 61 |

## Terms a newcomer met before they were defined

README: `gated` and `unhooked` as record values, "pattern floor", "rung one" and "rung two",
"the face". The first two screens of `docs/harness-spec.md`: D5, D8, D10, and half a dozen
ticket numbers pointing into `.scratch/hydra/`, which reads like scratch work a newcomer should
skip. `agy` is defined as Google's Antigravity CLI only on the spec's second screen, never in
the README or `llms.txt`. No ticket; the fix is a glossary and it is worth writing before the
repository is public.
