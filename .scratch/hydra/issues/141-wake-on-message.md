# 141 - Wake on message: who resumes a parent when its child ends

Type: grilling
Status: open
Blocked by: 390
Opened: 2026-11-04, by the session, from map 140's answer.

## The question

Infinity's idea, borrowed: a parent that waits on a child ends, and is resumed when the child
ends, instead of holding its process inside `delegate`. Each wake is a fresh run linked to the one
before, as `hydra resume` already makes, so the pid liveness rule holds (map 140, GPT and Opus).

What needs the human:

1. Who launches the resume: a typed `hydra wake`, `hydra status`'s reconciler, or a standing
   process.
2. Whether a resume with no human in the loop may spend quota, and how much.
3. How harness spec line 1131 ("Hydra needs no scheduler") and line 1272 ("nothing needed to be
   running") change.
4. Events are derived from records, not received (map 140 question 5). Whether that stays the rule.

Build 389 must be resolved first, since an automatic resume without it resets the depth cap. If
390's threshold does not hold, this closes as "dropped".
