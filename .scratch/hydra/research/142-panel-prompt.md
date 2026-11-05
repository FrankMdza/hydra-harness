# Decision panel for map ticket 142

You are one seat on a decision panel. Read only: do not edit, create or delete a file in the
repository, and do not commit. Never open, print or copy the contents of a credential file; paths
only, and do not repeat credential paths in your memo beyond naming the files that contain them.
Scratch work under `/tmp` is fine. Your seat name is in your task line. The human decides on the
panel's recommendation, so commit to one option per question and say why.

The human is Frank, a data analyst in Medellín. The repository is the portfolio piece for harness
engineering roles; `.scratch/` is the map, the build tickets and the research behind every
decision, and is part of what a reviewer reads.

## Read

1. `.scratch/hydra/issues/142-the-last-decisions-before-the-repository-is-public.md`.
2. The Answer of `.scratch/hydra/issues/134-the-public-gate.md` and `research/134-synthesis.md`.
3. `.scratch/hydra/labeling/2026-09-29/task-classes.jsonl`, a few lines only, to judge what the
   private-repo summaries disclose. `git grep -l private-repo` for the rest.
4. `research/139-post-quantum-route.md` sections 3 and 4, and `research/139-synthesis.md`.
5. `SECURITY.md`, `README.md`, `.gitignore`.

## Questions

1. **`.scratch/` and private-repo.** (A) Publish as it is. (B) Remove or redact the private-repo
   material and rewrite history (`git filter-repo`), then force-push before the switch. (C) A new
   public repository from one clean snapshot, the private one kept with its full history. (D)
   Publish without `.scratch/`. Say what a reviewer loses or gains under each, what private-repo
   discloses (strategy, tickers, money, nothing), and what is irreversible.
2. **Credential paths.** The human leans to generalising. (A) Leave them. (B) Generalise in every
   file: classes such as "the SSH private keys" or "gcloud's credential store" instead of this
   Mac's names. (C) Generalise only the names that are not standard CLI locations (custom key
   names, the extra provider directory), keep the standard ones. Does it matter once the history
   is public under option 1's choice? Does `tests/gate.rs` need its paths?
3. **The SSH line.** Should the human add `KexAlgorithms ^mlkem768x25519-sha256`, in `Host *` or per
   host? What happens with a server that does not offer it, and how the human checks the
   negotiated algorithm before and after (`ssh -v`, the line to look for). Any host in the
   human's use (GitHub, `gcloud compute ssh` to Debian or Ubuntu images, a VPS provider)
   where it could break, from documentation.
4. **The signing crate.** `aws-lc-rs` or `fips204` for ML-DSA-65 in build 382, with Ed25519 beside
   it. Weigh: a C and CMake build on the Mac and on Linux CI, audit and verification status,
   constant-time claims, maintenance, binary size, and that Hydra has no crypto dependency but
   `sha2` today. Check `Cargo.lock` and `deny.toml`.
5. **The order to the switch**, as a numbered list the human can follow, with who does each step.

Check at least two claims of the ticket against the repository and say which.

At most 900 words. End with `## Pending from me`, `## Changed` and `## Found`.
