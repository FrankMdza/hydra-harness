# 64 - Delegation from inside a run, once the kernel denies the corpus to that run

Type: grilling
Status: resolved
Blocked by:

## Question

Map ticket 62 put the macOS seatbelt around every recorded launch, and the human chose on
2026-09-21 to keep the corpus inside the kernel's denial: a run's process tree cannot read or
write `~/hydra/runs/`, whatever the spelling. The seatbelt is inherited by every descendant, and a
delegated child, harness spec 9, is Hydra itself launched from inside the run through the face:
`hydra run --parent <id>` reads the parent's record to check it is live and writes its own record
into the corpus. Measured the same day: under a corpus-denying profile `hydra status` fails with
"Operation not permitted" and the first read of a parent record fails the same way. So the face
does not work under the seatbelt, and the human took that over a corpus the kernel leaves open.

1. **Where does a delegated launch run?** The only process outside the sandbox that belongs to
   the run is the watcher, the detached half that owns the pty and the record. A face that asks
   the watcher to launch the child, over the pty or a pipe the watcher already holds, keeps the
   corpus denied to the model's tree and gives Hydra its own write. D5 forbids a second channel
   between the harness and Rungs; whether a pipe between a run's face and its own watcher is
   that channel, or plumbing inside the harness, is the question.
2. **Or does delegation move out of the run entirely?** The orchestrating session has launched
   every child so far, through `hydra run`, from outside. The face has been used on two runs in
   the whole corpus. If nobody delegates from inside, the honest answer is to say the face is
   not available under the seatbelt and leave the design as it is.
3. **What does the record say?** `run_start.sandbox` names the profile. A face call refused
   because the child cannot write its record should be a refusal the parent's record carries,
   in Hydra's words, not an EPERM in a tool result.

Until this closes, `SECURITY.md` and `IMPLEMENTATION.md` say the face is not available under the
seatbelt, and a delegated launch from inside a macOS run fails on the corpus read.

## Answer

Resolved 2026-09-22 with the human. **The face is not available under the seatbelt, and it says
so in Hydra's words.** Delegation stays with the orchestrator outside the run, which is how every
child in the corpus was launched; the face was used on two runs in its whole history. A
`delegate` call from inside a macOS run answers a tool error with a `hydra/` code naming this
ticket rather than an EPERM out of a corpus read, so the parent's record carries the refusal as
the tool result the provider streamed back. Moving the launch to the watcher was declined as a
second channel between a run and Hydra that D5 would have to rule on, for a feature nobody uses;
leaving it open was declined because the record should say what happened. Build ticket 76.

