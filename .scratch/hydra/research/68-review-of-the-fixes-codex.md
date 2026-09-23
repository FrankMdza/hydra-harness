3 findings

1. Explicit null inputs escape the new object check

src/policy/gate.py:743 treats an absent tool_input and an explicit JSON null identically. Under permissive, {"tool_name":"Task","tool_input":null} is allowed. Likewise, agy's {"toolCall":{"name":"wait","args":null}} is allowed because src/policy/gate.py:679 forwards null into the same branch. This contradicts the requested present-but-not-object refusal and the new explanation at src/policy/gate.py:676. These were also allowed at 0c91055, so this is incomplete hardening, not a widened permission.

Fix: distinguish absence from null using membership or a sentinel. Preserve missing agy args as an absent field or default them to {}, while retaining explicit null for rejection. Test null, booleans, numbers, strings and arrays, alongside missing fields.

2. Foreign envelope keys are rejected only when object-valued

src/policy/gate.py:630 recognizes toolCall only when its value is a dict. A Claude payload {"tool_name":"Task","tool_input":{},"toolCall":[]} is allowed under permissive, as are corresponding null or scalar variants. Cursor has the same gap. The new docstring at src/policy/gate.py:623 claims that carrying the other provider's key is enough to refuse it, which exceeds the implementation. This answers question 5. It does not bypass evaluation of the ordinary tool_name, and the example was already allowed at the baseline.

Fix: use key presence for foreign toolCall rejection and add malformed-envelope cases for Claude and Cursor. Keep interpretation of an actual agy envelope separate.

3. MCP respelling probes ask a gate that does not protect their target

tests/gate.rs:1581 creates a fresh permissive fixture, but the arguments at tests/gate.rs:1585 and tests/gate.rs:1587 still use root from the earlier standard fixture. Each fixture has a separate data root (tests/corpus/mod.rs:90). Consequently these two allow assertions would still pass if MCP path normalization or assembly were implemented correctly: that other fixture's corpus is outside this gate's deny list. The canonical control at tests/gate.rs:1596 uses permissive.runs, so it does not validate the same target.

Fix: construct both respelled arguments from permissive.root and compare against the canonical path under that same root.

Questions with no findings

Question 1: No newly refused legitimate provider payload found. The fixtures at tests/corpus/mod.rs:239, :315, :332 and :504 agree with the dialect branches at src/policy/gate.py:666 and :673. Codex uses the Claude hook dialect (src/provider/codex.rs:159). The added fetch floor is an intended restriction.

Question 2: No previously denied payload now allowed found. Comparing evaluate and as_tool_call against 0c91055, previous envelope reinterpretations are either preserved for their own dialect or refused; the other decision changes add denials (src/policy/gate.py:726, :746, :849).

Question 3: No new payload-triggered exception escaping the live answer path found. JSON parsing retains its error handling (src/policy/gate.py:1023), evaluation retains its exception guard (:960), and batch environment mutation is absent from main_single (:1033).

Question 4: No model-controlled selection of batch mode on the live hook found. Selection requires argv[2] (src/policy/gate.py:1076); the generated hook fixes argv to the dialect (src/policy/mod.rs:968). Model-controlled recorded arguments do reach batch evaluation during replay, intentionally, through src/policy/replay.rs:158 and :210; that does not authorize live execution.

Validation: ran the requested log and diff, compared both Python versions with in-memory payload probes, and ran cargo test --release --test gate: 93 passed. No source files edited.
