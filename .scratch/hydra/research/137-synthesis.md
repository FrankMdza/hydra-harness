# 137 synthesis: the lab's next arc

Panel of 2026-11-04, sat because the human asked for one on the five design documents, the cloud
plan and the post-quantum layer. Seats under `standard`, read only, from
`research/137-panel-prompt.md`:

- Opus, `claude-opus-5-5`, run `01a1a824-51a6`, 392k tokens, memo `137-seat-opus.md`.
- GPT, codex `gpt-6-astra`, run `01a1a824-b8f9`, 146k tokens, memo `137-seat-gpt.md`.
- Gemini, agy `gemini-3.8-flash-high`, run `01a1a825-55c3`, 653k tokens, memo `137-seat-gemini.md`.
- Grok, cursor `grok-4.7-high`, run `01a1a825-0eb8`, ended `auth_failed`, as on 2026-11-01. Three
  seats, no split on any main question, so no judge sat.

The human decides. Nothing below is resolved until they do.

## What the session checked after the panel

- Hydra opens no TLS connection anywhere. Its one HTTP client refuses any host but `127.0.0.1`,
  `localhost` or `[::1]` (`crates/records/src/loopback_http.rs:53-55`, `:99-103`). Opus's claim
  holds, and it settles question 3's option C: there is no Hydra connection to put
  X25519MLKEM768 on. Traffic to GCP goes through `gcloud`.
- The gate runs as Python under `/usr/bin/python3` (`src/policy/replay.rs:39`). Opus flagged this
  as unchecked. It means `tree-sitter-bash` cannot go into the gate as a Rust crate. It needs
  either a Rust helper the gate calls or a parser the system Python can run without packages, and
  that choice is the first thing the ticket has to settle.
- Both ML-DSA crates the seats named exist and are pre-1.0: `ml-dsa` 0.1.2 (RustCrypto) and
  `fips204` 0.5.0. RustCrypto's `ml-kem` is 0.3.4. Audit status was not checked for any of them.
- GCP holds no instance, disk or bucket today (`gcloud compute instances list`, `disks list`,
  `storage buckets list`, all empty).
- `config/export-allowlist.json` exists and holds `/Users/USER/hydra`. All three seats found
  that map 77's Answer still says the file does not exist.
- GPT noted that build 294's Outcome is empty. The Kev numbers the digest used are in
  `research/109b-kev.md`, uncommitted, not in the ticket. Both are true. The ticket is half done.

## Question 1, the cloud: A, three of three

Training only, on spot GPUs that are deleted at the end of the round. Hydra, the agents and the
vendor logins stay on the Mac. B and C put the CLIs' OAuth logins on a machine that stays up,
keep records on Google's disk past one round, and run a cost the budget only warns about. C also
gives up Seatbelt. All three seats name the same condition for moving to a B without vendor
logins: batch work that the Mac cannot finish, such as amd64 task images of several GB. In that
case it would be a spot VM per night, local models only, deleted when the night ends.

GPT added a rule for the cap: treat 10 USD as total spend, cleanup and storage included, and read
the real bill, because round one's 0.25 USD is still an estimate.

## Question 2, the experts: A, three of three

One dense base under about 1.5B on the Mac and one LoRA adapter per narrow task, trained in the
cloud, with Hydra's router (`system-one` until build 294 gives Kev a number) choosing the adapter.
It is the only option that fits 8 GB beside Kev's 3.0 GB. OLMoE at 4.2 GB plus Kev is 7.2 GB
before the OS. A merged sparse MoE grows total parameters with each expert, and total parameters
set the RAM. "Fits" is a plausible estimate. Nobody has measured it yet, and Opus's first ticket
does that.

This reads the human's "like MoE" at the level of tasks: the router is Hydra, the experts are
adapters of a few MB on disk, and only the ones in use are in memory. True sparse MoE stays a
cloud study under map 133's `expert_profile`, and is not served on the Mac.

## Question 3, post-quantum: provenance first, three of three

| Seat | Ranking | Crate for A |
|---|---|---|
| Opus | A, B, D, C | `ml-dsa` |
| GPT | A, C, B, D | `ml-dsa` |
| Gemini | A, B, C, D | `fips204` |

A is an ML-DSA signature over a manifest of hashes for everything a round makes, checked on the
Mac before anything loads. It is the first threat this machine has: a file from a VM being loaded
here. All three agree on B's limit: the GPU needs plaintext to train, so encrypting to a key on
the Mac protects a stored bundle, not the training step. B becomes useful once an export bundle
is stored between rounds. C has no target in Hydra's code (checked above). D, a narrow expert that
checks a repository for non-post-quantum crypto, fits the human's access goal and 133's
verifiable-reward answer, but it comes after the training path works.

The open point is where the signing key lives. Opus generates it on the VM per round and reads
the public half from the serial console before training. GPT warns that a key the training code
can reach signs whatever that code produces, so a signature proves origin and integrity, not
that the run was honest. The ticket has to state that limit.

## Question 4, the documents' ideas

| Idea | Opus | GPT | Gemini | Result |
|---|---|---|---|---|
| `tree-sitter-bash` in the gate | map | map | fog | map, starting with how the Python gate reaches a parser |
| Kev as Guardian | fog | fog | map | fog, until 294 step 3 |
| Verify-on-stop | map | map | fog | map, first as which providers have a stop hook |
| Short index in place of the long CLAUDE.md | map | map | map | map, the human's file |
| Routing by role and cost (doc 3) | drop | map, measured only | drop | drop; map 124 already holds the defaults |
| Novelty reward (doc 4) | fog | fog | drop | fog; the dynamic sampler is the verifiable version |
| Rust compute, Go control (doc 2) | drop | drop | drop | drop |

## Question 5, the first tickets

The seats agree on the content and differ on the order. Every seat has the signed manifest. Two
have a round two with one adapter. Opus puts a memory measurement first, GPT a preflight with no
spend, and Gemini a check on what the export carries. The session's order, which keeps the work
that costs nothing first:

1. **Preflight, no spend.** Read round one's real bill. Do build 290's leftover precondition,
   which `training/round1/README.md` names: read the eligible tasks with fenced code for content
   from outside `~/hydra` before an export with tasks leaves the machine. State the round's budget
   with cleanup included. Done when every number has a source or is marked unknown.
2. **Base plus adapters beside Kev, measured.** A dense base under 1.5B with two adapters swapped
   on a loopback server, next to Kev and one agent: steady and peak footprint, swap, time per
   swap, against thresholds written before the run. If it does not fit, question 2's A dies here.
3. **Signed round manifest.** The VM writes SHA-256 hashes and an ML-DSA-65 signature, and the Mac
   refuses on a flipped byte, a wrong key or a missing signature. The crate is chosen by NIST's
   known-answer vectors, not by vote. The round-one adapter, replayed through it, loads.
4. **Round two, one adapter.** One narrow task with a verifiable reward, trained on a spot VM,
   signed by 3, loaded through 2, routed by `system-one`. It works if a per-tool held-out score
   beats an always-Bash baseline, which round one could not show, and the bill reads under 1 USD.

Build 294 steps 2 and 3 stay ahead of 2 and 4, because they decide the router.

## What the human decides

- Questions 1 to 3 as voted, or not.
- Map 133's question 1 (dense first, or OLMoE first). This panel's answer to question 2 points to
  dense, but 133 reserved that choice for the human.
- Whether the short index replaces the repo's CLAUDE.md, which is the human's file.
- Whether to fix Cursor's login so the Grok seat sits again.
