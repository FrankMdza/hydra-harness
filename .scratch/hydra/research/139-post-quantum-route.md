# 139 - The post-quantum route, and what it protects

Research for map ticket 139. Every source was read on 2026-11-04 and is numbered at the end.
Measurements on this Mac, the same day, are marked "measured". No credential file was opened: the
checks were `ssh` with `PreferredAuthentications=none` and `openssl s_client` handshakes.

The short answer: the lab's real exposure is that bearer tokens and private keys sit in files that
any process of Frank's user can read, a gated Hydra run under `standard` included. That is a
classical problem, and the fix is classical: the Keychain, shorter sessions, and a read deny in the
sandbox profile. Post-quantum work is worth doing for two narrower things, data that leaves the
machine and must stay secret for years, and signatures over what the lab trains and ships. For
those, "our own system" should mean a small Hydra format built on `age` and on one vetted ML-DSA
crate, never a primitive we write.

## 1. The threat model, per asset

"Harvest now, decrypt later" is the post-quantum threat to encryption: traffic or files recorded
today and opened once a large quantum computer exists. RFC 9958 recommends hybrid key exchange
against it [1]. For signatures the threat is forgery later, which only matters for a signature that
must still be trusted then. NIST's final IR 8547, published 2026-10-27, sets the schedule for
retiring quantum-vulnerable public-key algorithms [2]; the dates 2030 for deprecation and 2035 for
disallowance come from secondary reports of it [3], since the page I read held only the abstract.

| Asset | Classical threat | Post-quantum threat | Which matters here |
|---|---|---|---|
| Tokens and keys listed in the ticket | A process of the same user reads the file and acts as the account until revoked | None in the near term. A bearer token is not encrypted to anyone; a quantum computer adds nothing to reading a file | Classical, and it is the largest one |
| Run records in `~/hydra/runs/` | Read by another process; a gated run under `standard` can read them by shell (section 5) | Only if they leave the machine | Classical. They stay local |
| Exports (preference pairs) | Leaking a secret inside a pair; Hydra already audits for planted secrets | Recorded in transit or at rest, opened later | Both, once exports go to a cloud VM |
| Training data on a GCP VM | The GCP account token above; a misconfigured bucket | Same as exports. TLS to Google already negotiates a hybrid group (measured, below) | Classical first; at rest, a PQ envelope is cheap to add |
| Weights coming back | Swapped or altered on the way, then loaded and run | A forged signature only matters if the signature must hold in 2035 | Integrity, so a signature. Classical Ed25519 suffices today; ML-DSA adds the long horizon |
| The skill store | A run that writes it edits every later run's instructions; Hydra already denies that write [src/policy/mod.rs] | Same as weights, if snapshots are ever signed | Classical, and already covered by the write deny |

Measured on the wire: `oauth2.googleapis.com` and `compute.googleapis.com` negotiated
`X25519MLKEM768` with the local OpenSSL 4.1.0. `api.github.com` negotiated plain `X25519`. GitHub's
SSH server offers `mlkem768x25519-sha256` first, but the local `ssh` (Apple's OpenSSH 10.0p2)
negotiated `ecdh-sha2-nistp256`, because its own default list puts that first. Neither
`/etc/ssh/ssh_config`, its `.d` file, nor a user config names `KexAlgorithms`; `~/.ssh/config` does
not exist. Upstream OpenSSH 10.0 made `mlkem768x25519-sha256` the default [4], so Apple's build
departs from upstream here. One line in a new `~/.ssh/config`, `KexAlgorithms ^mlkem768x25519-sha256`,
would move it first. That is Frank's file to write. GitHub enabled the method on 2026-10-14 for
github.com outside its US data-residency region [5].

## 2. What "our own system" should mean

Compose, never write a primitive. The sources point the same way, though none of them says "do not
roll your own" in those words.

- NIST's assurance comes through validated modules. AWS-LC FIPS 3.0 was the first library with
  ML-KEM in a FIPS 140-3 validation [6], and NIST lists the module [7]. A home-written ML-KEM
  could never get there.
- SP 800-227, final on 2025-09-18, covers how to use a KEM safely and how to combine a classical
  and a post-quantum KEM so that an attacker must break both [8]. That is exactly the composition
  a Hydra format would do, and it is specified.
- RFC 9958, June 2026, is written for engineers who use public-key cryptography in their
  software, and recommends hybrids [1].
- RustCrypto's own READMEs for `ml-kem` and `ml-dsa` still say "The implementation contained in
  this crate has never been independently audited!" [9][10]. If the maintainers of the main
  pure-Rust code say that, a lab-written implementation would be worse.
- A real case: `ml-dsa` before 0.1.0-rc.3 leaked through a hardware division on secret-derived
  values, RUSTSEC-2025-0144, CVE-2026-22705, fixed with a constant-time Barrett reduction [11].
  Correct-looking code from good maintainers had a timing leak. The lab has no way to find that
  class of bug in its own code.

So "our own" means a Hydra-specific format and tool: which files get sealed, to which recipients,
with what manifest, signed by which key, checked where. The primitives stay in vetted crates, and
the format reuses an existing envelope wherever one fits.

## 3. What exists to reuse

**`age`.** Version 1.3.0 (27 Dec 2025) added native post-quantum recipients, "based on HPKE with a
hybrid ML-KEM-768 KEM", with recipients starting `age1pq1` and `age-keygen -pq` to make one [12].
The latest is 1.3.2, 29 Aug 2026 [12]. A hybrid recipient can only be mixed with other
post-quantum recipients, so a file cannot be silently weakened by a classical one [13]. The
recipient strings are about 2,000 characters [13]. The Rust port `rage` is at 0.12.1, 14 Jul 2026,
and its release notes add `age1tag1` and `age1tagpq1` hardware recipients but no `age1pq` [12].
So only the Go binary does post-quantum `age` today. Forks such as `anubis-age`
use ML-KEM-1024 and do not interoperate [14].

**X-Wing.** `draft-connolly-cfrg-xwing-kem-11`, 23 Sep 2026, is an individual draft "not endorsed
by the IETF" with no intended RFC status [15]. Apple ships it anyway as `XWingMLKEM768X25519` in
CryptoKit [16]. It is a well-reviewed construction with no standing as a standard yet.

**TLS.** RFC 10024, August 2026, Proposed Standard, defines `X25519MLKEM768`, codepoint 0x11EC
[17]. `rustls` 0.23.27 (5 May 2025) prefers post-quantum key exchange by default [18]. Google
Cloud load balancers turned it on by default after October 2026 for client-facing TLS, with a
`DEFERRED` opt-out until October 2027 [19]. Google's API front ends already negotiate it
(measured, section 1). Chrome has used it by default since version 131 [20].

**SSH.** RFC 10042, August 2026, Informational, defines `mlkem768x25519-sha256` among three hybrid
methods, all "SHOULD" [21]. OpenSSH 10.0, 9 Apr 2025, made it the client default [4].

**ML-DSA signing tools.** No equivalent of `age` for post-quantum signing turned up. `minisign`
and `ssh-keygen -Y sign` are classical. The options are a library (section 4) or CryptoKit's
`MLDSA65` with the key held in the Secure Enclave [16]. That last one needs Swift.

## 4. The Rust crates

Versions are from the crates.io API, read 2026-11-04 [22].

| Crate | Latest | Audit or verification | Tests and constant time | Maintenance |
|---|---|---|---|---|
| `ml-kem` (RustCrypto) | 0.3.4, 31 Oct 2026 | README: never independently audited [9] | README makes no KAT or constant-time claim; the repo has `tests/` and `checkct/`, which I did not open [9] | Active |
| `ml-dsa` (RustCrypto) | 0.1.2, 31 Oct 2026 | Never audited [10]; the timing advisory above, fixed [11] | Same as `ml-kem` | Active |
| `fips204` (IntegrityChain) | 0.5.0, 23 Oct 2026 | No audit claim found | Claims constant-time key generation and signing within one attempt, at source level, checked by manual review and dudect; tests NIST ACVP and Wycheproof vectors [23] | Active, one vendor |
| `libcrux-ml-kem`, `libcrux-ml-dsa` (Cryspen) | 0.0.11 each, 7 Oct 2026 | Formally verified with hax and F* for panic freedom and functional correctness [24]; the ML-KEM portable and AVX2 code is verified [25] | README: binaries "are _not_ verified to be side-channel resistant" [24] | Pre-release, every crate below 0.1 [24] |
| `aws-lc-rs` | 1.19.0, 28 Oct 2026 | Wraps AWS-LC, whose FIPS module includes ML-KEM [6][7] | ML-DSA moved from the `unstable` module to stable `signature::ML_DSA_*` in 1.18.0 [26]; whether ML-DSA is inside the FIPS scope is not stated, and `rustls-webpki` says ML-DSA-44 is not [26] | Active, AWS |
| `pqcrypto-*` | n/a | Unmaintained: RUSTSEC-2026-0164, -0161, -0166, because PQClean is archiving; they point to `ml-kem` and `ml-dsa` [27] | n/a | Do not use |
| `oqs` | 0.11.0, 1 May 2025 | Upstream liboqs: "WE DO NOT CURRENTLY RECOMMEND RELYING ON THIS LIBRARY IN A PRODUCTION ENVIRONMENT" [28] | n/a | Stale for 18 months |

Recommendation: build on `aws-lc-rs` for ML-DSA signing and, if the lab ever needs a raw KEM, for
ML-KEM. It is the only option with a validated module behind it, a stable API, and a large
maintainer. It is a C library underneath, which costs build time and a C toolchain. For encryption, do not touch a KEM crate at all in version one: shell out to
`age`. Keep `libcrux-ml-dsa` as the cross-check: sign with one, verify with the other in a test,
which catches an encoding mistake in either. The RustCrypto crates are a reasonable second choice
once one of them is audited.

## 5. Protecting the tokens

**What Hydra's sandbox denies today**, read from `src/sandbox.rs` and `src/policy/mod.rs`. The
seatbelt profile starts with `(allow default)` (`src/sandbox.rs:590`). On top of that it denies
writes to Hydra's own paths (`denied_write_paths`, `src/policy/mod.rs:794`): the policies, the
config files, the checkout, the runs, pruned, reservations, puppets, approvals, the skill store
and snapshots, the parent git directory and sibling worktrees and spawns. It denies reads of the
corpus, the spawns directory, the pruned set and the approvals (`src/sandbox.rs:465-475`). Only a
policy with a bounded shell also denies `file-read-data` under Hydra's root and checkout, which on
this machine is `~/hydra` (`bounded_read_denies`, `src/sandbox.rs:941`), so `~/hydra/.env` is
unreadable there. A gated interactive session additionally gets every write under the home
directory denied (`copilot_block`, `src/sandbox.rs:854`), but reads stay open. `policies/floor.json`
holds command patterns for git, aws and slack and names no credential path.

Nothing in any profile denies reading `~/.ssh`, `~/.codex/auth.json`, `~/.config/gcloud`,
`~/.config/gh` or `~/.cursor`, and under `standard`, with its unbounded shell, `~/hydra/.env` is
readable too.

**What each CLI allows.**

- `gh` stores its token "securely in the system credential store" and falls back to "a plain text
  file" only if that fails, or when `--insecure-storage` is passed [29]. A mode 600 `hosts.yml`
  may hold only the user name; `gh auth status` would say, and was not run. A gh maintainer noted that anyone who can run `security` can read the Keychain item
  [30], so the Keychain stops a file reader and does little against a shell running as Frank.
- Codex has `cli_auth_credentials_store`, with `file` and `keyring` in the English docs and an
  `ephemeral` mode in the localized ones [31]. Changing it does not move an existing `auth.json`;
  a fresh login is needed [32] (third-party, unverified).
- Claude Code already uses the encrypted macOS Keychain, and falls back to
  `~/.claude/.credentials.json`, mode 0600, only when the Keychain refuses the write [33].
- Cursor's CLI docs do not say how `cursor-agent` stores its login [34]. The ticket's mode 644
  file is unexplained.
- Google Cloud has a Google Cloud session control, set by a Workspace admin per organizational
  unit, from 1 to 24 hours, which covers `gcloud` and ADC user credentials, and is off by
  default [35]. `gcloud auth application-default revoke` and `gcloud auth revoke` remove local
  credentials [36]. Whether Frank's account is a Workspace account that can set this was not
  checked.
- SSH keys: a passphrase plus the macOS agent. CryptoKit's post-quantum Secure Enclave types are
  ML-KEM and ML-DSA [16], which OpenSSH does not use for authentication.

First, a read deny in the seatbelt profile for every
policy, on `~/.ssh`, `~/.codex/auth.json`, `~/.config/gcloud`, `~/.config/gh`, `~/.cursor`,
`~/hydra/.env` and `~/.claude/.credentials.json`. One caveat: each provider must still read its
own credential, so a run's provider process needs its own file and only that one. A seatbelt
profile applies to the whole process tree, so the deny cannot exempt the provider while catching
its shell; that conflict needs a decision (see Found). Second, move `codex` to `keyring` and
confirm `gh` uses the Keychain. Third, turn on the Google Cloud session control if the account
allows it. None of this is post-quantum.

## 6. The route

**Version one, two weeks.**

1. The read deny of section 5 for the paths no provider needs (`~/.ssh`, `~/.config/gcloud`,
   `~/.config/gh`, `~/hydra/.env`), with a test in the style of `tests/sandbox_read_only.rs` that
   reads each one from a gated shell and expects EPERM. Then verify by reverting, as the lab
   already does: drop the rule and watch the read succeed.
2. A `hydra seal` that wraps the Go `age` binary with `age1pq1` recipients for exports and training
   data before they leave for a VM, sealed to the VM's own recipient and the Mac's. The Mac's
   identity lives in the Keychain, never in a file a run can read.
3. A signed round manifest for build 382: a canonical JSON list of every file's SHA-256 plus the
   base model, round and date, signed with ML-DSA-65 through `aws-lc-rs`, and a test that verifies
   the same signature with `libcrux-ml-dsa`. Weights coming back are verified against the manifest
   before anything loads them. Sign with Ed25519 alongside it, so the manifest is a hybrid and
   stays valid if one scheme falls.
4. The one-line `KexAlgorithms` change for SSH, written by Frank.

After that: move the signing key into the Secure Enclave through a small Swift helper over
CryptoKit's `MLDSA65` [16]; move to `rage` once it ships `age1pq`; revisit the RustCrypto crates
once one is audited.

## Sources, all read 2026-11-04

1. RFC 9958, Post-Quantum Cryptography for Engineers, June 2026. https://www.rfc-editor.org/rfc/rfc9958.html
2. NIST IR 8547 final, 27 Oct 2026. https://csrc.nist.gov/pubs/ir/8547/final
3. CSA research note on IR 8547 final. https://labs.cloudsecurityalliance.org/research/csa-research-note-nist-ir8547-pqc-transition-20261030-csa-st/
4. OpenSSH release notes, 10.0, 9 Apr 2025. https://www.openssh.com/releasenotes.html
5. GitHub changelog, 22 Sep 2026. https://github.blog/changelog/2026-09-22-security-improvements-for-ssh
6. AWS Security Blog, AWS-LC FIPS 3.0. https://aws.amazon.com/blogs/security/aws-lc-fips-3-0-first-cryptographic-library-to-include-ml-kem-in-fips-140-3-validation
7. NIST CAVP/CMVP entry for AWS-LC. https://csrc.nist.gov/projects/cryptographic-algorithm-validation-program/details?product=18881
8. NIST SP 800-227 final, 18 Sep 2025. https://csrc.nist.gov/pubs/sp/800/227/final and https://csrc.nist.gov/news/2025/nist-publishes-sp-800-227
9. RustCrypto `ml-kem` README. https://github.com/RustCrypto/KEMs/tree/master/ml-kem
10. RustCrypto `ml-dsa` README. https://github.com/RustCrypto/signatures/tree/master/ml-dsa
11. RUSTSEC-2025-0144. https://rustsec.org/advisories/RUSTSEC-2025-0144 and https://osv.dev/vulnerability/RUSTSEC-2025-0144
12. age releases, https://github.com/FiloSottile/age/releases ; rage releases, https://github.com/str4d/rage/releases
13. `filippo.io/age` package docs. https://pkg.go.dev/filippo.io/age
14. `anubis-age` docs. https://docs.rs/crate/anubis-age/latest
15. X-Wing datatracker, revision 11. https://datatracker.ietf.org/doc/draft-connolly-cfrg-xwing-kem/
16. Apple, quantum-secure workflows in CryptoKit. https://developer.apple.com/tutorials/data/documentation/cryptokit/enhancing-your-app-s-privacy-and-security-with-quantum-secure-workflows.md
17. RFC 10024, formerly draft-ietf-tls-ecdhe-mlkem. https://datatracker.ietf.org/doc/draft-ietf-tls-ecdhe-mlkem/
18. rustls releases, 0.23.22 and 0.23.27. https://github.com/rustls/rustls/releases
19. Google Cloud, post-quantum TLS for load balancers. https://docs.cloud.google.com/load-balancing/docs/post-quantum-tls
20. Google Security Blog, A new path for Kyber on the web, Sep 2024. https://security.googleblog.com/2024/09/a-new-path-for-kyber-on-web.html
21. RFC 10042, August 2026. https://www.rfc-editor.org/rfc/rfc10042.html
22. crates.io API for `ml-kem`, `ml-dsa`, `fips204`, `libcrux-ml-kem`, `libcrux-ml-dsa`, `aws-lc-rs`, `oqs`. https://crates.io/api/v1/crates/
23. `fips204` docs. https://docs.rs/fips204
24. libcrux README. https://github.com/cryspen/libcrux
25. `libcrux-ml-kem` docs and Cryspen's write-up. https://docs.rs/libcrux-ml-kem and https://cryspen.com/post/ml-kem-verification/
26. `aws-lc-rs` 1.18.0 `unstable/signature.rs`, and `rustls-webpki` 0.103.10 `aws_lc_rs_algs.rs`. https://docs.rs/crate/aws-lc-rs/latest/source/src/unstable/signature.rs and https://docs.rs/crate/rustls-webpki/0.103.10/source/src/aws_lc_rs_algs.rs
27. RUSTSEC-2026-0164, -0161, -0166. https://rustsec.org/advisories/RUSTSEC-2026-0164 , https://osv.dev/vulnerability/RUSTSEC-2026-0161 , https://rustsec.org/advisories/RUSTSEC-2026-0166
28. liboqs README via `oqs-sys`. https://docs.rs/crate/oqs-sys/latest/source/liboqs/README.md
29. gh manual, `gh auth login`. https://cli.github.com/manual/gh_auth_login
30. cli/cli PR 7043. https://github.com/cli/cli/pull/7043
31. OpenAI Codex authentication docs. https://developers.openai.com/codex/auth and https://developers.openai.com/de-DE/docs/auth
32. Classmethod, Codex credentials to the Keychain (third-party). https://dev.classmethod.jp/articles/codex-cli-credentials-store-keyring/
33. Claude Code IAM docs. https://code.claude.com/docs/en/iam
34. Cursor CLI docs. https://cursor.com/docs/cli
35. Google Workspace, set session length for Google Cloud services. https://knowledge.workspace.google.com/admin/security/set-session-length-for-google-cloud-services
36. Google Cloud, ADC docs. https://docs.cloud.google.com/docs/authentication/provide-credentials-adc

## Pending from me

- The panel prompt, `research/139-panel-prompt.md`, is not written; this run was told to write one
  file.

## Changed

- Wrote `.scratch/hydra/research/139-post-quantum-route.md`. No other file touched, nothing
  committed.

## Found

- The Mac's `ssh` negotiates `ecdh-sha2-nistp256` with GitHub although both sides support
  `mlkem768x25519-sha256`. Apple's OpenSSH 10.0p2 lists nistp256 first with no config file asking
  for it. `KexAlgorithms ^mlkem768x25519-sha256` in `~/.ssh/config` fixes it; that file is Frank's.
- How a provider reads its own token while its shell cannot is a decision for the map (section 5).
- `aws-lc-rs` does not state its FIPS scope for ML-DSA, and whether Hydra already depends on it
  through `rustls` was not checked.
- Not checked: what `~/.cursor/cli-config.json` holds, whether `gh` keeps its token in the Keychain
  (`gh auth status` would say), whether Frank's Google account can set session control, and the
  `sshd` key exchange on the GCP VM images.
- The 2030 and 2035 IR 8547 dates come from a secondary source; the NIST PDF was not read.
