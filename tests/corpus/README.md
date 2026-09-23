# Attack corpus

One row per attack class from map ticket 68 research. A class with no probe and
no documented reason is absent. `not applicable` rows name why the class cannot
reach Hydra rather than carrying an empty test. An `Open` row is a class Hydra does not
answer; it has no probe because there is nothing yet to pin, and `SECURITY.md` states it.

| Class | Mechanism or reason | Probe |
|---|---|---|
| A1. Indirect injection through file content the run reads | Not preventable by a harness. What Hydra controls is what an injected run can do, which is the policy (`src/policy/gate.py`); the preamble's data-not-instructions line (`src/preamble.rs`) is advice | `tests/gate.rs`: policy matrix across five policies |
| A2. Injection through fetched web content | Actions bounded by a policy the run cannot widen (`src/policy/gate.py`) | `tests/gate.rs`: policy matrix across five policies |
| A3. Poisoned agent-instruction files in the checkout | Open. Providers load `CLAUDE.md`, `AGENTS.md`, `.cursorrules` and similar as configuration; Hydra neither strips nor records them. Stated in `SECURITY.md` | none |
| A4. MCP tool-description poisoning and line jumping | Admission per policy and per server (`src/mcp.rs`); `mcp_call` path denied under every policy (`src/policy/gate.py::evaluate`) | `tests/mcp.rs`; `tests/gate.rs`: MCP path denial |
| A5. MCP rug pull after launch | Open. Nothing pins what a server advertises at `tools/list`; the record carries server names only. Stated in `SECURITY.md` | none |
| A6. Hook and provider config rewritten inside the workspace | `protected_worktree_paths` (`src/policy/mod.rs`); existing hook config refused at spawn (`src/provider/mod.rs`); gate sha in hook config (`src/policy/mod.rs`) | `tests/gate.rs`: protected worktree paths; `tests/spawn.rs`: spawn refusal and tamper detection |
| A7. Config root redirected by the environment | Child environment is an allowlist of four names plus per-provider additions (`src/env.rs`) | `src/env.rs::passes_nothing_outside_the_allowlist`, `src/env.rs::drops_the_secrets_this_shell_exports`, `src/env.rs::agy_gets_the_base_four_and_the_proxies_and_nothing_else`; `tests/pty_tee.rs::the_child_gets_the_environment_it_is_handed_and_nothing_else` |
| A8. Secret inherited through the environment | Same allowlist; `GEMINI_API_KEY`, messaging socket, `NODE_OPTIONS`, `ANTHROPIC_API_KEY` dropped by construction (`src/env.rs`) | `src/env.rs::drops_the_secrets_this_shell_exports`; `tests/pty_tee.rs::the_child_gets_the_environment_it_is_handed_and_nothing_else` |
| A9. Symlink, `..` and path-assembly escape | `realpath` before matching (`src/policy/gate.py::resolve`); `inside()` prefix check (`src/policy/gate.py::inside`); macOS kernel resolves paths (`src/sandbox.rs`) | `tests/gate.rs`: symlink and `..` probes; `tests/spawn.rs`: `the_seatbelt_refuses_the_corpus_however_the_path_is_spelled` |
| A10. Command-validation bypass by respelling | Pattern floor (`src/policy/gate.py::floor_violation`); a shell facility carrying no command denied under every policy (`Capability::OpaqueShell` in `src/policy/table.rs`). Residual: shell does not interpret commands (`tests/gate.rs::a_subcommand_from_a_shell_substitution_is_refused_rather_than_evaluated`) | `tests/gate.rs`: floor matrix; `tests/gate.rs`: `a_shell_facility_carrying_no_command_is_denied_under_every_policy` |
| A11. Exfiltration through a pre-approved fetch target | Fetch argument naming a denied path refused, `file://` included (`src/policy/gate.py::evaluate`) | `tests/gate.rs`: fetch denial probes |
| A12. Invisible instruction smuggling | Delegated child text marked inside a boundary drawn after the child stopped (`src/face.rs`) | `src/face.rs::the_marker_is_present_and_the_token_was_not_in_the_text` |
| A13. Escalation by relaunching the harness with a wider policy | `HYDRA_RUN_ID` containment (`src/containment.rs`); `HYDRA_ROOT` refused from inside seatbelt (`src/sandbox.rs`, `src/run.rs`) | `src/containment.rs`: unit test; `tests/delegation.rs`; `tests/spawn.rs`: `hydra_root_is_refused_from_inside_a_seatbelt` |
| A14. Trust and identity confusion through a git worktree | Every Hydra run is a `git worktree` checkout; Claude Code's trust dialog is disabled for non-interactive `-p` launches (`src/provider/claude.rs`) | not applicable |
