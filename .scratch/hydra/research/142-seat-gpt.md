GPT seat: **1C, 2C, SSH under `Host *`, and `aws-lc-rs` for build 382.** These are recommendations for Frank’s decision.

**1. Publish a clean snapshot in a new repository, retaining sanitized `.scratch/`.**

[withheld: what the seat read in a private repository]

- **A:** Preserves complete provenance, but permanently exposes unrelated private-project details.
- **B:** Preserves much of the development sequence, but changes commit IDs and requires careful cleanup of every published ref. Reviewers gain history whose redactions need explaining.
- **C:** Preserves the engineering reasoning in sanitized tickets while keeping the original history private. Reviewers lose commit-by-commit verification of earlier development; disclose that explicitly.
- **D:** Removes much of the portfolio’s strongest evidence: decisions, failed approaches and acceptance records. Deleting `.scratch/` only at the tip also leaves its history accessible.

Choose C because its publication boundary is easiest to inspect. Remove private-project summaries, generalize useful examples, and retain Hydra-specific reasoning. Review screenshots, attachments and new untracked research too. Once public, copies cannot reliably be recalled; neither deleting files nor later rewriting history reverses disclosure.

**2. Generalize custom credential names; retain standard locations and meaningful test syntax: C.**

Standard CLI locations help readers reproduce security claims. Custom names add personal inventory without explaining Hydra. Apply this distinction throughout the snapshot, including `build/242`, `issues/58`, `research/68-attack-corpus-opus.md` and the new 139 material.

`tests/gate.rs:1288` contains a conventional SSH-key pathname in a command string passed to the gate. It tests refusal of a tilde-based path outside scope; it does not open the key. Keep that syntax, or substitute a synthetic filename with identical semantics. The actual personal filename is unnecessary.

Under C, unpublished history stays private, so snapshot cleanup matters. Under A, changing only current files leaves old names public. Generalization reduces disclosure; it does not protect credential contents.

**3. Add the preference under `Host *`.**

```sshconfig
Host *
    KexAlgorithms ^mlkem768x25519-sha256
```

The caret prepends to defaults; it does not require that algorithm. A server without it negotiates another shared method. Put specific host overrides earlier, since SSH generally uses the first obtained value. An older **client** that does not recognize the algorithm can reject the configuration. Check `ssh -Q kex` first. [OpenSSH configuration](https://man.openbsd.org/ssh_config#KexAlgorithms)

Before and after, run `ssh -v -T git@github.com`, `ssh -v USER@VPS`, and `gcloud compute ssh INSTANCE --ssh-flag=-v`. Look for **`debug1: kex: algorithm:`**, not merely the offered list. Google documents flag forwarding and `--dry-run`. [gcloud reference](https://cloud.google.com/sdk/gcloud/reference/compute/ssh)

GitHub documents support, except its US data-residency region. Debian/Ubuntu VM and VPS support depends on the installed server; older servers should fall back. No documented incompatibility identified here makes per-host configuration necessary. Actual VM negotiation remains unmeasured. [GitHub announcement](https://github.blog/changelog/2026-09-22-security-improvements-for-ssh/)

**4. Choose `aws-lc-rs`, without claiming ML-DSA certification.**

Its stable ML-DSA API and AWS maintenance outweigh introducing native code. Require ML-DSA-65 **and** Ed25519 verification over identical canonical, domain-separated manifest bytes. [AWS releases](https://github.com/aws/aws-lc-rs/releases)

The build premise needs correction: current upstream documentation requires a C/C++ compiler, but **not CMake for non-FIPS builds**, on both Mac and Linux. FIPS builds require CMake and Go. Verify the pinned release’s requirements. [Mac requirements](https://aws.github.io/aws-lc-rs/requirements/apple.html), [Linux requirements](https://aws.github.io/aws-lc-rs/requirements/linux.html)

`fips204` offers pure Rust, no unsafe code, vectors and source-level constant-time claims within each signing attempt, checked with review and dudect. Those are useful assurances, not an independent audit or binary-level proof. Its maintenance base is smaller. Likewise, AWS-LC’s validation and verification work must not be generalized to every ML-DSA implementation property. [fips204 documentation](https://github.com/integritychain/fips204)

Binary-size differences remain unmeasured: compare stripped release builds. `Cargo.lock` confirms neither candidate is present. Run `cargo deny` against the resulting lockfile; `deny.toml` checks both target platforms and all features.

**5. Order to publication**

1. **Frank:** Accept or revise these recommendations; complete build 378’s 30-minute acceptance.
2. **Authorized implementation session:** Prepare the sanitized snapshot and repair references.
3. **Frank:** Review disclosure edits and any protected configuration changes.
4. **Implementation session:** Scan the exact snapshot, build/test from a fresh clone, verify links and configure CI.
5. **Frank:** Create a new private repository; push only the clean snapshot, set topics and verify CI.
6. **Frank:** Inspect the hosted contents, then switch that repository public. Keep the original private.
7. **Frank/implementation session:** Measure SSH negotiation and implement 382 separately; neither expands the agreed publication gate.

## Pending from me

None for this read-only seat.

## Changed

No files changed; no credentials opened.

## Found

Verified 21 tracked files mention private-repo. Counts are now 716 `.scratch/` files of 1,667, one above the ticket’s figures. Confirmed the signing dependencies are absent; corrected the unconditional CMake assumption.
