# 142 synthesis: publish as it is, generalise only today's custom names, `Host *`, `aws-lc-rs`

Panel of 2026-11-04 on `issues/142-the-last-decisions-before-the-repository-is-public.md`. Seats
under `standard`, read only, from `research/142-panel-prompt.md`:

- GPT, codex `gpt-6-astra`, run `01a1aa00-1a3c`, memo `142-seat-gpt.md`.
- Gemini, agy `gemini-3.8-flash-high`, run `01a1aa01-b1ce`, memo `142-seat-gemini.md`.
- Opus, `claude-opus-5-5`, run `01a1aa03-5a66`, memo `142-seat-opus.md`.
- Grok not seated, Cursor's login.

## What the session checked

- **No tracked file names a custom credential path.** `git grep` for the custom key names, the
  extra provider directory and the gcloud and gh file names returns nothing. They are only in
  today's uncommitted map 139 files. Opus found it, and the ticket's list was wrong on this.
- **`ssh -G github.com` puts `ecdh-sha2-nistp256` first**, ahead of `mlkem768x25519-sha256` and
  `sntrup761x25519-sha512`, so Apple's default order is why GitHub gets the classical exchange.
  `~/.ssh/config` sets no `KexAlgorithms` today (grep, nothing printed).
- **The private-repo summaries.** [withheld: what the seat read in a private repository]
- `.gitignore:42` ignores `labeling/*/classes-*.jsonl`, and the files are still tracked, because
  the rule came after them.

## Question 1, `.scratch/`: A, two of three

Gemini and Opus: publish as it is. GPT: C, a new public repository from one clean snapshot.

- For A: `.scratch/` is what a harness-engineering reviewer reads, and 62 commit hashes cited in
  `.scratch/` and `IMPLEMENTATION.md` resolve in this repository (Opus). B and C leave them
  dangling. D breaks `README.md:164` and `:201`.
- For C: the private-repo material is about a different private project, and once public it
  cannot be recalled (GPT).
- What decides it is personal, and Opus says so: whether Frank minds what they disclose being public. If yes, C and not B, because C keeps the private history whole.

## Question 2, credential paths: C, two of three

GPT and Opus: generalise the custom names, keep the standard CLI locations, which every CLI's own
documentation names. Gemini: leave them. Since the custom names exist only in uncommitted files,
generalising them before their first commit keeps them out of history under A. `tests/gate.rs`
keeps `cat ~/.ssh/id_rsa`, a generic string the gate must refuse, three of three.

## Question 3, the SSH line: yes, `Host *`, three of three

The `^` puts the method first and keeps the rest of the default list, so a server without it
negotiates the next shared method. Opus adds `sntrup761x25519-sha512` second, so Debian 12 and
Ubuntu 24.04 images (OpenSSH 9.2 and 9.6, from memory, not checked) still get a post-quantum
exchange:

```
Host *
    KexAlgorithms ^mlkem768x25519-sha256,sntrup761x25519-sha512
```

A `Host` block above `Host *` that sets `KexAlgorithms` would win. Check before and after with
`ssh -v git@github.com 2>&1 | grep 'kex: algorithm:'`.

## Question 4, the signing crate: `aws-lc-rs`, two of three

GPT and Opus: `aws-lc-rs`. Gemini: `fips204`.

- For `aws-lc-rs`: one crate gives ML-DSA-65 and Ed25519, where `fips204` needs `ed25519-dalek`
  beside it (Opus). AWS maintains it. `aws-lc-sys` 0.45.0's licences fit `deny.toml` as it is
  (Opus, from the local registry). GPT read that non-FIPS builds need a C compiler and not CMake.
- For `fips204`: pure Rust, no C, smaller binary. One vendor, at 0.5 (Opus).
- Opus's condition: put it in its own workspace crate under `crates/`, so `hydra` itself stays on
  `sha2` alone. Binary size and CI time are unmeasured, three of three. Nowhere may say Hydra's
  signing is FIPS-validated.

## Question 5, the order to the switch

1. Frank: build 378's 30-minute session. This session writes both files too, so either it sits
   idle and no other `claude` runs, or the check reads the sandbox's denials instead of the shas.
2. Session: generalise the custom names in the uncommitted 139 files.
3. Frank: the export allowlist, untracked or left (protected config).
4. Session: commit today's tree, rerun gitleaks over the whole history, CI green.
5. Frank: push, then switch to public.
6. Any time, independent: the SSH line. After the switch: build 382 on `aws-lc-rs` in its own
   crate.

## What the human decides

- Question 1: A, or C if what the summaries disclose should stay private.
- Questions 2 to 4 as above.
