# 62 - Should containment be enforced by reading argv, or by the operating system?

Type: grilling
Status: open
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
