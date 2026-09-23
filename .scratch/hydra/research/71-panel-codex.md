I recommend a fifth option: keep the links and give Codex a Hydra-owned, snapshot-only reader through MCP, authorized by the existing file_read grant rather than general MCP permission.

Mechanism. Standard already grants snapshot reads (policies/standard.json:6), injection creates symlinks (src/skills/mod.rs:445), and Codex can receive stdio MCP configuration (src/provider/codex.rs:208). Add a protected, per-run reader with read_skill(name, relative_file, offset, limit). Bind it at launch to the pinned snapshot and injected names. Reject traversal, absolute paths, symlink escapes and nonregular files; open read-only with race-safe containment and bounded output. Never execute skill scripts. Add new src/skills/reader.rs functions serve and read_skill, approximately 200–300 lines including protocol handling.

Change run in src/run.rs:382 and MCP plan/write_config/record in src/mcp.rs:205,350,405 to inject and record this reserved server only for Codex runs granted snapshot reads, approximately 80–120 lines. Reject server-name collisions and pin its executable and manifest outside writable worktree content. Existing Codex MCP config is already among protected_worktree_paths (src/policy/mod.rs:682).

Change capability_of, target_paths and evaluate in src/policy/gate.py:607,184,730, plus src/policy/table.rs:349, approximately 60–100 lines: recognize only the reserved reader before generic MCP classification, derive the target from its bound manifest, and apply file_read scope checks. Ordinary MCP calls remain unchanged. This distinction is necessary because MCP admission currently does not grant permission (src/mcp.rs:18), and generic MCP arguments encounter protected-path rejection (src/policy/gate.py:836). The server must independently enforce its bound scope. Add approximately 150–250 lines of containment, spoofing, policy-denial and provider integration tests; document the alternate reading route in the launch instructions. These are design estimates, not a patch.

Provider costs. Codex gains a server process, tool-schema context and an unfamiliar reading route; measure discovery and actual hook payloads before shipping, since its table still records incomplete live verification (src/policy/table.rs:341). Claude and cursor-agent keep their Read tools unchanged (src/policy/table.rs:92,159). Agy keeps view_file; run_command still cannot read protected paths (src/policy/table.rs:240; src/policy/gate.py:878). No provider receives broader shell permission. This does not solve Codex's general read-only-policy support, currently refused at preflight (src/provider/codex.rs:32).

Hostile-run delta. Codex can now retrieve and repeatedly request admitted skill bytes through an authorized tool. Intended new authority is reading only; bounded responses limit individual requests, not aggregate consumption. A containment or server-identity bug could expose other files or misclassify a malicious tool. Do not rely on seatbelt to repair that: its write defence is separate from gate capability decisions (src/sandbox.rs:3).

The strongest objection is substantial protocol and identity machinery for a small delivery defect, plus a new trusted reader to audit. Nevertheless, it preserves lazy access and avoids either mutable instructions or shell exceptions.

Option 1: Copies permit instruction tampering and require explicit reap exclusion, including staged copies; reap calls commit_everything (src/reap.rs:296). Cleanup cannot undo mid-session tampering.

Option 2: Reject the proposed path-only exception. Shell checks deliberately refuse reads and writes together (src/policy/gate.py:270); kernel protection is not authorization.

Option 3: Honest temporary degradation, but sacrifices Codex skills because its reads use Bash (src/policy/table.rs:346).

Option 4: Explains failure without restoring access; injection still produces the same links (src/skills/mod.rs:445).
