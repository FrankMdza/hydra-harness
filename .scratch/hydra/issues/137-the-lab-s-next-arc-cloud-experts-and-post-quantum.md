# 137 - The lab's next arc: training in the cloud, experts that come home to the Mac, and the post-quantum layer

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-11-04, by the session, from the human's request of that day to review five design
documents and the cloud plan with a panel. The documents and the state of the lab are digested in
`.scratch/hydra/research/137-sources-digest.md`. Map 133 holds the MoE question and stays the
place where its open questions 1, 2, 4 and 5 are answered; this ticket decides the arc around it
and must not redo 133's answered question 3, nor maps 77, 88, 109, 111 or 112.

## Question

1. **What goes to the cloud, and in which order.** (A) Training only, on ephemeral spot GPUs,
   with Hydra and every agent staying on the Mac. (B) A, plus a small Linux host running Hydra
   headless under bubblewrap for long batch work such as the apprentice's nightly bank, with the
   Mac as the console. (C) Hydra moves to a cloud Linux host and the Mac becomes a thin client.
   Each has to say what it does to the vendor CLIs' logins on a remote machine, to the 10 USD cap,
   and to the rule that records leave the Mac only for the human's GCP project and are deleted
   after the round.
2. **How the experts come home, "the way MoE is used today".** (A) One dense base on the Mac plus
   one LoRA adapter per narrow task, trained in the cloud, with a router in Hydra (the
   `system-one` classifier or Kev) choosing the adapter. (B) Experts trained separately from one
   seed in the cloud, then merged into one sparse MoE with a trained router, one file coming home.
   (C) An existing open MoE such as OLMoE-1B-7B with chosen experts fine-tuned under a frozen
   router, map 133's second option. Say which fits 8 GB beside Kev's 3 GB.
3. **Where post-quantum cryptography enters first.** (A) Provenance: an ML-DSA (FIPS 204)
   signature on every adapter, weight file and export bundle made in the cloud, checked on the
   Mac before anything loads. (B) At rest: what leaves the Mac is encrypted to a key that stays
   on it, hybrid ML-KEM-768 plus X25519, then ChaCha20-Poly1305. (C) In transit: the hybrid TLS
   group X25519MLKEM768 on Hydra's own connections to the cloud. (D) A narrow expert of the lab's
   own that checks a repository for key exchange and signatures that are not post-quantum, trained
   on verifiable rewards. Rank them and name the crate.
4. **Which ideas from documents 1 to 4 enter the map now, which go to the fog, which are
   dropped.** Parsing shell commands with `tree-sitter-bash` in the gate; Kev as a cheap
   "Guardian" that rates an action's risk; verify-on-stop guards; a short AGENTS.md-style index
   in place of the long CLAUDE.md; routing by role and cost from document 3; a novelty reward from
   document 4 for choosing bank tasks or research topics; the Rust compute and Go control split.
5. **The first three tickets, in order,** given build 294 is claimed and half done.

Decided by the human on the panel's recommendation. The panel prompt is
`.scratch/hydra/research/137-panel-prompt.md`.

## Panel

Sat 2026-11-04, three seats (Grok `auth_failed`). Synthesis and votes in
`.scratch/hydra/research/137-synthesis.md`: 1A, 2A and 3A, all three of three. Waiting on the human.

## Answer

Resolved 2026-11-04 by the human on the panel's recommendation (`research/137-synthesis.md`).

1. **A.** Training only, on spot GPUs deleted at the end of the round. The human asked whether
   that means the weights come back and none of the agents' records stay in the cloud. Yes, with
   one precision the session gave: the training data has to be on the VM while the GPU trains,
   so the records are there for the length of the round, on the VM's disk only, and the disk is
   deleted with the VM. Round one used no bucket. What comes back is the adapter and the report.
2. **A as the first step, and from scratch reopened as research.** The human wants nano LLMs of
   the lab's own, trained from zero in Rust, with retraining of chosen parts of the weights, and
   asked for research and a panel. That is map 138. Base plus adapters stays the first
   measurement because it is the cheapest way to learn what fits on the Mac.
3. **A, and an own post-quantum security system as research.** The human wants a system of the
   lab's own to protect its things, and believes key theft does not apply because everything runs
   through OAuth. The session's correction is in map 139: an OAuth refresh token on disk is a
   bearer key. That is map 139.
4. **As voted.** Map now: `tree-sitter-bash` in the gate, verify-on-stop, the short index. Fog:
   Kev as Guardian, the novelty reward. Dropped: role routing from document 3, the Rust and Go split.
5. **As the synthesis orders them**, builds 380 to 383, with 382 (the signed manifest) blocked by
   map 139 so that its design comes from that answer.
