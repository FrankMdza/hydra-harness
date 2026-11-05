# Seat Opus, map 140

## 1. The short and map 138

None of map 138's votes change. The corrected reading makes weight-space programming credible for networks under 10M parameters. But the blog's own limit still holds for language models: an edit "could not add an ability the model lacked". Map 138 trains the gate classifier from zero. An edit can only fix a model that already exists, so it cannot come before builds 384 or 385.

There is one cheap experiment, and it belongs as an extra arm of 386, not as a new ticket. After 385, take one class of false allows on `tests/corpus/` and try a single direct weight edit on the gate model, such as subtracting one direction from chosen MLP columns. Measure it the way 386 measures frozen layers and LoRA: gain on the target class, and forgetting on the held-out set. It runs locally on the Mac's CPU and costs $0.

Who writes the edit is the human's open item, "may a frontier model write weight diffs". Until the human says yes, a human or a script writes it from activations. The session does not.

## 2. Infinity: A

- **A, borrow ideas only.** Costs $0 and no RAM, and leaves the contract alone until a map ticket decides otherwise.
- **B, a sliced runtime.** Deferred, per question 4.
- **C, Infinity itself in AWS.** No. Infinity keeps histories in Aurora DSQL, which contradicts map 137's answer. It bills Bedrock or an API key per completion, and Lambda has no pty, so none of the four routed CLIs would run there.

The RAM gain from A is smaller than the ticket implies. Wake on message frees only a parent that is waiting. A child that is working keeps its 200 MB process either way. Today's orchestrator is an interactive session, and `src/resume.rs:50` refuses to resume an interactive run. So the only parents that could be put to sleep are recorded runs that delegate, and nobody has counted how many there are.

## 3. Which idea first: wake on message, measured before it is decided

The reserved-suffix event cannot come first. It needs input delivered mid-run, and section 10 says "No attach, no input". Breaking that is a D5 decision. The stable id per input fixes no duplicate that I found.

Wake on message does not break the liveness rule, provided each slice is a run of its own. Each run still has its own pid and start time. "Waiting" can be derived from records as a parent that ended on a `Handle` result while its child is still alive, with no sidecar file, so D10 holds.

It does touch two sentences. Section 9 says "Hydra needs no scheduler, only a cap on live children". Section 10's worked scenario ends "nothing needed to be running". Something has to launch the resume when the child ends: a typed `hydra wake`, `hydra status`'s reconciler, or a standing process. That choice is a map decision. Each automatic resume also spends quota with no human in the loop.

## 4. B: after map 138 has a model that calls tools

The gate classifier does not call tools, so B waits until after 386 and after a model that does.

Write the loop rather than taking Infinity's crates as a library. `infinity-agent-core` 0.2.0 depends on tokio, async-trait, rhai and rap-client, and rap-client pulls in reqwest and hyper. Hydra's `Cargo.lock` has none of these. I checked: `grep '^name = "tokio"' Cargo.lock` returns 0. Borrow the trait shape instead. A local model does fit it: `CompletionModel::stream` wrapped in `SingleModelProvider` (`crates/infinity-provider-protocol/src/lib.rs:85-104`), which closes the research's open gap.

Keep the history as linked runs in Hydra's own record, not in Infinity's store, so D5's single contract survives.

## 5. Security

Events should be derived, not received. A wake reads the child's record itself, and only the process that owns a record appends to it. Every run's profile already denies reads of the corpus (`src/sandbox.rs:465`). So a locally derived event is as authentic as the record it comes from, and needs no new signature. Text from the child that reaches the woken parent is marked, as `face::marked` already does.

What map 139 has to add:

- The waker launches through `run::run`, so the environment allowlist and 387's credential deny apply with no second path.
- If an event ever crosses machines, it carries 382's dual signature.
- No RAP endpoint, and no network listener of any kind.

One gap matters as soon as resume is automated. `resume.rs` passes `parent: None`, so a resumed depth-2 child comes back at depth 0 and can delegate twice more. That is acceptable today only because a human typed the resume.

## 6. Tickets

1. **Build, measure the waiting parents, no change to `src/`.** From the records: how many recorded runs called `delegate`, the peak number of parents waiting at once, and the RAM-seconds they held at 200 MB each. A test feeds the tally a fixture corpus of two parents whose overlap is known and asserts the figures. Write the threshold before running it: if the peak times 200 MB is under 400 MB, wake is dropped.
2. **Build, resume keeps the depth.** Compute depth through the `resumed_from` chain as well as `parent_run_id`, so no record field changes meaning. In `tests/delegation.rs`, a hand-built depth-2 child is resumed and its `delegate` is refused at the cap. Then revert the change and watch the refusal disappear.
3. **Map, wake on message.** Blocked by 1. It decides who wakes a parent, and how the two sentences in section 9 and section 10 change.

Claims I checked against the source:

- `receive_event__injected` is refused when the model calls it. Confirmed at `event_processor.rs:1798`, which returns an error and loops back, with a test at line 3040.
- `infinity-agent-core` is at version 0.2.0, edition 2024. Confirmed.
- A local model fits `ModelProvider`. Confirmed, as in section 4.
- The research quotes resume as "fails loudly" for a provider with no resume concept. In the code that branch is dead: `supports_resume` returns `true` for all four providers (`src/provider/mod.rs:271`). The refusals that actually fire are for a live run and an interactive run.

## Pending from me

Nothing.

## Changed

Nothing in the repository. I downloaded two Infinity source files to `/tmp/r140opus/`.

## Found

- A resumed run gets `parent: None` (`src/resume.rs`), which resets the depth cap. It is harmless while a human types every resume, and a hole once anything resumes runs automatically.
- `supports_resume()` is `true` for every provider, so spec 10's "fails loudly" has no case that reaches it.
- The interactive orchestrator cannot be resumed, so wake on message cannot save the session's own RAM.
- Infinity's core crate would add tokio, hyper, reqwest and rhai to a binary that has none of them.
- `SingleModelProvider` makes a local nano model a drop-in, which closes research 140's gap.
