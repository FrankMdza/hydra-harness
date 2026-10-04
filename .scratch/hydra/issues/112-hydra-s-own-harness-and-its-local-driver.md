# 112 - Hydra's own harness, and its local driver

Type: grilling
Status: open
Blocked by:
Opened: 2026-10-04, by the session, from map 111: map 88's condition is met.

## Question

Map 88 decided the shape of a harness of Hydra's own (a separate Rust child executable, the four
tools under `claude`'s names so `gate.py` is unchanged, every call through `hook_command` failing
closed, frames shaped like `claude`'s, print mode only) and deferred it until a local model emits
tool calls on 7 of 10 tasks. Map 111 measured four that do.

1. **Where the endpoint URL and the model name live**, the decision map 88 left: a run that can
   rewrite them picks its own model. (A) a protected config, human-written, like `panels.json`;
   (B) flags on `hydra run`, recorded in `run_start`, never read from the worktree; (C) both, the
   config as the allowlist and the flag as the choice within it.
2. **The default local model**: `qwen2.5:3b` (smaller) or `ministral-3:3b` (faster), once their
   licences are read; the licence decides which one the training round of map 109 may fine-tune.
3. **First use**: the apprentice's bank (map 87) through the harness, so its attempts act under
   the gate instead of answering in prose.

## Answer
