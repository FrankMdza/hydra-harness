# 62 - Should containment be enforced by reading argv, or by the operating system?

Type: grilling
Status: resolved
Blocked by:

## Question

An adversarial review on 2026-09-14 found five holes in the gate. One is fixed, one is a
case-folding mismatch, and the other three are the same bug in three spellings: the gate is a
string matcher standing in front of a shell, and `standard` hands that shell to the model on
purpose. A path can be spelled through an interpreter, through a variable, through a hard link,
or through a tool that carries no command string at all, and each spelling needs its own patch
to a floor whose failure mode harness spec 7.4 already names out loud.

The reviewer's own recommendation, quoted because it is the clearest statement of the
alternative: run children under a different uid, with `~/hydra` owned by the human, the
worktree and the curate checkout group-writable, and the corpus, the store, the spawn directory
and the policy directory not writable by that uid at all. Then `sh -c`, a hard link, a variable
and a ligature all fail on the same `EACCES`, and the gate goes back to what it is good at,
which is shaping capabilities and writing down what was asked.

1. **Does the second uid happen?** It is the first thing in this design that needs something
   from the machine rather than from Hydra: a user to create, an installer step, and a story
   for what a person's own `~/.claude` credentials look like from that uid, since D6 says the
   vendor CLI signs in as the human and section 4.3 measured that a wrong `HOME` logs three of
   four providers out. That measurement is the hard part, not the file modes.
2. **If it does not, what is the gate honest about?** `SECURITY.md` says the gate is a floor
   and not a boundary. Three of five findings say the floor has holes a patch closes one at a
   time. Either the document says the floor is best-effort against a non-adversarial model, and
   the record is the real product, or the project keeps patching spellings and the next review
   finds the next one.
3. **What does the record promise either way?** The record says what the gate was asked and
   what it answered. That stays true under both designs and is the thing nobody else has. It is
   worth deciding whether Hydra's claim is containment, or evidence, or both, before the
   repository is public and somebody decides for it.

## Answer

Resolved 2026-09-21 with the human. The research run on `agy` (01a0c5ab-9a7b) recommended no
second uid and a `SECURITY.md` that calls the gate best effort. The session measured a third route
the research did not consider, and the human took it.

1. No second uid. What happens instead is the operating system's own sandbox on macOS,
   `sandbox-exec`, around the vendor binary under the same user with the same `HOME`, so the
   keychain and the logins are untouched. Measured 2026-09-21 in a scratch directory: a direct
   write, a write through a variable, a write from Python and a write through a symlink into a
   denied subpath all fail with EPERM; a hard link to a denied file cannot be created from
   inside; the denied directory cannot be renamed; and `claude`, `cursor-agent`, `agy` and `codex`
   all start underneath it, `codex` still logged in. The one path that reached a denied inode
   was a hard link the human made outside the sandbox first, which is not the threat.
2. The gate is honest about being a floor for capabilities and a writer of evidence. The writes
   to Hydra's own directories, harness spec 7.5's deny list, and the reads of the corpus are
   denied by the kernel, whatever the spelling; the gate keeps deciding which capabilities a
   policy grants and recording every call. `SECURITY.md` says both halves in those words.
3. The record promises evidence of what the gate saw, and the sandbox promises that Hydra's own
   directories were not written by a run. Containment of the machine at large is not claimed.

The cost is stated: macOS only today, and Apple marks the tool deprecated while shipping it in
26 and while Anthropic's own sandbox runtime uses it. Linux is a port through landlock or bwrap.
Build ticket 68.
