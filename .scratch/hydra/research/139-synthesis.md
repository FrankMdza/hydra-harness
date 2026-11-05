# 139 synthesis: a post-quantum system of our own, and the tokens first

Panel of 2026-11-04 on `research/139-post-quantum-route.md`. Seats under `standard`, read only,
from `research/139-panel-prompt.md`:

- GPT, codex `gpt-6-astra`, run `01a1a86b-ccbf`, 139k tokens, memo `139-seat-gpt.md`.
- Gemini, agy `gemini-3.8-flash-high`, run `01a1a86d-70c7`, 600k tokens, memo `139-seat-gemini.md`.
- Opus, `claude-opus-5-5`, run `01a1a86f-2b1c`, 490k tokens, memo `139-seat-opus.md`.
- Grok not seated, Cursor's login.

No main question split badly enough to need a judge. The human decides.

## What the session checked

- **Run records are not shell-readable under `standard`.** The research's asset table says they
  are. `rules_of` pushes `ReadDenySubpath(spawn.corpus)` for every policy (`src/sandbox.rs:465`),
  and `corpus` is `layout.runs()` (`src/spawn_config.rs:162`). Opus found it. The research's
  section 5 was right and its table was wrong.
- **bwrap cannot deny a single file the way the profile does today.** `ReadDenySubpath` renders
  as `--tmpfs <path>` (`src/sandbox.rs:795`), which mounts over a directory. A deny on
  `~/hydra/.env` or `~/.codex/auth.json` on Linux needs a file rendering, such as binding
  `/dev/null` over it. Opus found it. It matters because the cloud VM runs Linux.
- **`claude` reads its token through `/usr/bin/security`.** `src/env.rs:157-162` refuses a PATH
  without `/usr/bin` for that reason. So no profile can keep Claude's Keychain item from the
  claude run's own shell without breaking `claude`.
- Gemini and Opus found that neither `age` nor `rage` is installed. Gemini and Opus found that
  `aws-lc-rs`, `ring` and `rustls` are not in `Cargo.lock`, so `aws-lc-rs` is a new dependency
  with a C build.
- Opus found a third key directory under `~/.ssh`, which the ticket's list missed.

## Question 1, order: SSH line, then the read deny, three of three

All three put the one-line `KexAlgorithms ^mlkem768x25519-sha256` in `~/.ssh/config` first, since
it costs one line, and the human writes it. All three call the credential read deny the item that
protects the most. On what comes after them:

- Gemini: manifest, then seal.
- GPT: seal, then manifest, with neither blocking the other.
- Opus: manifest, and seal cut from version one. Google traffic already negotiates
  `X25519MLKEM768`. GCP disks are encrypted with symmetric keys. The GPU needs plaintext anyway.
  So a seal guards against a leaked account or a misconfigured bucket, which are classical
  threats and covered by the token work.

The session's reading: the manifest comes third, two of three. Seal is two to keep against one to
cut, and nobody puts it ahead of the token work. It costs nothing to leave it last, and nothing
in build 383 waits on it.

## Question 2, the credential conflict: deny now, measure the Keychain, then migrate

GPT and Gemini voted C (migrate to the Keychain and deny the files). Opus voted A narrowed:
deny every credential path except the launched provider's own, with B as a measurement and not a
dependency. On the facts they agree more than the letters say:

- All three deny `~/.ssh`, `~/.config/gcloud`, `~/.config/gh`, `~/hydra/.env` and
  `~/.claude/.credentials.json` in every profile, and every other provider's token file.
- All three say the Keychain does not separate a provider from its shell. GPT: "Keychain storage
  alone does not create a provider/shell boundary". Gemini: exposure moves to Keychain IPC.
  Opus: `claude` itself reads through `security`.
- So the migration's gain is unmeasured. Opus's second ticket and GPT's disposable Keychain item
  are the same measurement.

The order all three accept: the deny first, with its test. Then measure what is still reachable
through the Keychain. Then migrate `codex` and `gh` only where that measurement shows a gain.

The test, merged from GPT and Opus: dummy files at every path under a scratch `HOME`, never a real
credential. A gated shell gets EPERM on each one and the provider reads its own. Then revert the
rule and watch the read succeed, under every policy and on both macOS and Linux. One more
assertion records whether a disposable Keychain item, or `security find-generic-password` by exit
code only, succeeds from the gated shell. That turns the known gap into a test result.

The cost Opus names: under the deny, `gcloud` and `gh` work moves to the ungated session.

## Question 3, encryption: Go `age` with `age1pq1` recipients, three of three

A Rust composition over a KEM puts the bugs in the combiner, the KDF and the nonces, where the
lab cannot see them. Waiting for `rage` delays protection. Rust stays in the orchestration: a
`hydra seal` that pins the `age` binary, parses recipients, refuses a classical mix and publishes
atomically. GPT checked the age docs: the research's "only Go" is too broad, because the
documented post-quantum plugin works with compatible implementations. `age` is not installed
today.

## Question 4, signing, three of three

- `aws-lc-rs` signs with ML-DSA-65, and `libcrux-ml-dsa` verifies in a test only, both against
  NIST ACVP vectors. `fips204` is the pure-Rust fallback if the C build is unwelcome (Opus).
- Ed25519 beside it, and **both signatures required**, because an either-signature rule loses the
  point (GPT). The canonical, domain-separated manifest binds schema, algorithms, key ids, round,
  base-model digest and each artifact's path, length and hash.
- On the VM: a fresh key pair per round under a `signer` user separate from `trainer`. It signs
  only after training exits, and the Mac pins the public key over `gcloud compute ssh` before
  anything is uploaded (Opus). The Mac signs the round's input manifest so the VM can check what
  it trains on.
- What it cannot prove, three of three: honest training. A signer reachable from training is a
  signing oracle.
- Do not infer FIPS validation for ML-DSA from AWS-LC's ML-KEM validation (GPT).

## The tickets that follow, in order

1. **The human's `~/.ssh/config` line**, with negotiated key exchange recorded before and after
   for GitHub and the VM image.
2. **Build 387, the credential read deny.** A new rule both composers read, with a file rendering
   on bwrap. The scratch-`HOME` EPERM test, the revert, and the Keychain assertion.
3. **Build 388, measure what stays reachable, no code.** What `gh auth status` says, the keys (not
   values) of `~/.cursor/cli-config.json`, whether a `codex` keyring item prompts from a gated
   shell, and the VM image's `sshd` key exchange. Migrate where it shows a gain.
4. **Build 382, amended** with question 4's design, and unblocked.
5. **`hydra seal`, last**, built when data is stored somewhere other than a VM disk deleted with
   its round.

## What the human decides

- Questions 1 to 4 as above, and whether seal stays last or goes to the fog.
- Writing the `~/.ssh/config` line.
- Whether `aws-lc-rs`'s C build is acceptable, or `fips204` signs instead.
