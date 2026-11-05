## Seat Opus, map 142

**1. `.scratch/` and private-repo: A, publish as it is.**

I read the summaries in `labeling/2026-09-29/`. [withheld: what the seat read in a private repository]

What each option costs:

- **A** keeps everything a reviewer can check. The labeling corpus is evidence that Hydra ran real work for a real project, and the tickets cite commits. I resolved the hex strings in `.scratch/` and `IMPLEMENTATION.md`: 62 distinct ones are commits in this repository.
- **B** rewrites every hash, so those 62 citations dangle. The force-push also destroys the private history unless a mirror is kept. And 21 files mention private-repo, `map.md` among them, so the redaction is a pass over prose, not just deleting four files.
- **C** dangles the same 62 citations, and a public repo that starts with a single commit reads like a dump. The private history survives, so this is the reversible fallback.
- **D** breaks `README.md:164` and `:201`, which point into `.scratch/`. It also takes away the part a harness-engineering reviewer reads to see how decisions were made.

The only irreversible step is the switch itself, under any option. If Frank does not want what they disclose public, take C, not B.

**2. Credential paths: C.**

Measured: no tracked file names the custom ones. `git grep` over the tracked tree finds no a second ed25519 key, `google_compute_engine`, `credentials.db` or `gh/hosts.yml`. What is tracked are standard CLI locations: `~/.codex/auth.json` in `issues/58`, `~/.ssh/config` in `build/242`, `~/.ssh` and `~/.ssh/known_hosts` in `research/68`. Those names are in every CLI's documentation, so they tell an attacker nothing.

The custom names appear only in today's uncommitted 139 files. Generalise them there before their first commit and they never enter the history, which is why the history question is moot under A. Full generalisation (B) means rewriting old tickets for no gain.

`tests/gate.rs` keeps its paths: `cat ~/.ssh/id_rsa` at line 1288 is a generic string the gate must refuse.

**3. The SSH line: yes, in `Host *`.**

Measured here: `ssh -G github.com` lists `ecdh-sha2-nistp256` first, ahead of `mlkem768x25519-sha256`. That is why Apple's build negotiates classical key exchange, and `ssh -Q kex` shows that both post-quantum methods are compiled in. The `^` prefix moves the listed methods to the front and keeps the rest of the default list. A server that offers neither method negotiates the next one both sides share, so nothing breaks.

I suggest `KexAlgorithms ^mlkem768x25519-sha256,sntrup761x25519-sha512`. As I know the release history (I did not check it today), Debian 12's OpenSSH 9.2 and Ubuntu 24.04's 9.6 have sntrup761 but not mlkem, so the second entry still gets post-quantum key exchange from those GCE and VPS images. Debian 13 and GitHub offer mlkem.

ssh uses the first value it obtains, so if any `Host` block above `Host *` already sets `KexAlgorithms`, that block wins.

To check, before and after:

- `ssh -G github.com | grep -i kexalgorithms` shows the list the client will offer.
- `ssh -v git@github.com 2>&1 | grep 'kex: algorithm:'` should print `debug1: kex: algorithm: mlkem768x25519-sha256`.
- On a VM, `gcloud compute ssh VM --ssh-flag=-v`.

I know of no host among these that would refuse a connection because of the line.

**4. Signing crate: `aws-lc-rs`, in its own workspace crate.**

- **One crate:** it provides both ML-DSA-65 (stable since 1.18.0) and Ed25519. `fips204` would need `ed25519-dalek` beside it, which makes two new crypto trees.
- **Licence:** I read `aws-lc-sys` 0.45.0's licence expression from the local registry. It uses only ISC, Apache-2.0, MIT, BSD-3-Clause and MIT-0 alternatives, so the current `deny.toml` allow list should pass without an edit.
- **Build:** `aws-lc-rs` 1.18.1 is already in `~/.cargo/registry`, so this Mac has built it before. Its build dependencies include `cc` and `cmake`. GitHub's Ubuntu runners have a C compiler. Measure CI time before claiming anything about it.
- **Audit and FIPS:** neither crate has an audit claim. `fips204` claims constant time and tests against ACVP and Wycheproof vectors, but it has one vendor and is at 0.5. AWS-LC's FIPS module covers ML-KEM, and whether it covers ML-DSA is not stated. Say nowhere that Hydra's signing is FIPS-validated.
- **Size:** binary size is unmeasured. Putting the dependency under `crates/`, beside `crates/grader`, keeps `hydra` itself on `sha2` alone and keeps the C build out of anyone who never runs a round.
- **Cross-check:** keep `libcrux-ml-dsa` as a test-only verifier, as synthesis 139 says.

**5. The order to the switch**

1. Frank: type build 378's 30-minute gated acceptance session and compare both files' sha before and after.
2. Session: generalise the custom key names, the third key directory and the gcloud file names in the uncommitted 139 files, then commit today's tree.
3. Session: rerun gitleaks over the whole history, including that commit.
4. Frank: set the eight topics from synthesis 134 if not already set, with no homepage.
5. Frank: push master.
6. Frank: switch the repository to public.
7. Frank, any time and independent of the switch: add the SSH line and record the `kex: algorithm:` line for GitHub and one VM, before and after.
8. Session, after the switch: build 382 on `aws-lc-rs` in its own crate.

**Claims checked against the repository**

- "`.scratch/` is 715 of 1,666 tracked files": 716 of 1,667 today, which is close enough.
- "Apple's OpenSSH negotiates `ecdh-sha2-nistp256`": `ssh -G` confirms the order behind it.
- "`tests/gate.rs` names concrete credential paths of this Mac": false. It names `~/.ssh/id_rsa` and `~/.claude/` paths, which are generic.
- The four task-class files are tracked although `.gitignore` lists `classes-*.jsonl`. The ignore rule came after the files were added, so it does not untrack them.

## Pending from me

Nothing. Two things are unmeasured: binary size and CI build time with `aws-lc-rs`, and the OpenSSH versions on the Debian and Ubuntu images, which I gave from memory.

## Changed

Nothing in the repository. I wrote `/tmp/shas.txt` and `/tmp/hx.txt`.

## Found

- No tracked file names a non-standard credential path. Only today's uncommitted 139 files do.
- 62 commit hashes are cited in `.scratch/` and `IMPLEMENTATION.md`, and B or C leaves them dangling.
- `classes-*.jsonl` are ignored by `.gitignore` but still tracked.
- `aws-lc-sys` 0.45.0's licence expression fits the current `deny.toml`.
- `README.md` links into `.scratch/`, which rules out D.
