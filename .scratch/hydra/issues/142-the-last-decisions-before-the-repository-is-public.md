# 142 - The last decisions before the repository is public, and two from map 139

Type: grilling
Status: open
Blocked by:
Opened: 2026-11-04, by the session, from the human's answer after map 140.

## The human's words

Translated from Spanish: "4, let the panel choose [what to do with `.scratch/`]. 5, I would
generalise [the credential paths], but tell me what a panel thinks. After that we publish? [...]
The SSH line and the crate for signing (aws-lc-rs or fips204): I want a panel to help me decide
this."

## What stands, measured 2026-11-04

- `FrankMdza/hydra-harness` is private. Map 134's list is done except the human's 30-minute
  acceptance session of build 378 and the switch itself. gitleaks over the history finds nothing
  beyond fingerprinted fixtures.
- `.scratch/` is 715 of 1,666 tracked files, 7.1 MB. 21 tracked files name `private-repo`, the
  human's other private project, among them the four task-class files under
  `labeling/2026-09-29/`, whose lines summarise private-repo tasks. History keeps them since
  2026-09-29, so removing them from the tree does not remove them.
- Concrete credential paths of this Mac are named in tracked files (`build/242`, `issues/58`,
  `research/68-attack-corpus-opus.md`, `tests/gate.rs`) and in today's uncommitted tickets 139 and
  its research, which list a third key directory under `~/.ssh`, a second ed25519 key, `google_compute_engine`,
  `~/.codex/auth.json`, `~/.config/gcloud/credentials.db`, `~/.config/gh/hosts.yml`, modes
  included. Contents were never read.
- `config/export-allowlist.json` names `/Users/USER/hydra`. `config/README.md` already says the
  file is per machine and a clone writes its own (map 130).
- SSH: Apple's OpenSSH 10.0p2 negotiates `ecdh-sha2-nistp256` with GitHub although GitHub offers
  `mlkem768x25519-sha256` (research 139). The proposed line is `KexAlgorithms
  ^mlkem768x25519-sha256` in `~/.ssh/config`.
- Signing for build 382: `aws-lc-rs` (C build, AWS-LC, FIPS validation covers ML-KEM, not ML-DSA)
  or `fips204` (pure Rust). Research 139 section 4 and the 139 synthesis question 4.

## Panel

Sat 2026-11-04, three seats (Grok not seated, Cursor login). Synthesis in
`research/142-synthesis.md`. The session generalised the custom key names in the 139 and 142
files, seat memos included, before their first commit. Waiting on the human.
