# 70 - Does the delegate face serve more than one call at a time?

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-22, from the study session's block (g), question 2.

## Question

The delegate face (`src/face.rs`, harness spec 9) reads stdin one request at a time and blocks up
to 600 seconds on a `delegate` call, so a parent that asks for two children gets them in series.
Each `delegate` already writes its own record through its own watcher, so dispatching without
blocking the stdin loop adds no second writer to any record. What it changes is the cap: harness
spec 8's concurrency cap of eight counts live runs, and today a parent can hold at most one child
open through the face. A concurrent face lets one parent fill the cap by itself.

1. Keep the face serial, and say in spec 9 that it is serial on purpose.
2. Dispatch concurrently, with a per-parent limit below the global cap.
3. Dispatch concurrently and let the global cap be the only limit, which depends on build ticket
   81 making the cap a reservation rather than a check.

## Answer

Resolved 2026-09-22 with the human: the face stays serial, on purpose. Harness spec 9 says so in
one sentence. A concurrent face lets one parent fill the cap of eight by itself, the face refuses
inside the seatbelt since map 64 so it is rarely reached on macOS, and a parent that wants children
in parallel is an orchestrator, which launches them from outside. No build ticket.
