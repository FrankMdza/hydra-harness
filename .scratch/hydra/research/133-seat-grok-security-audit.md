# Security audit of SECURITY.md

`SECURITY.md` is current on the holes it names, and stale on the export redactor. Open tickets name holes it does not. Closing all of them is not where the tree stands.

Refused, not retried the same way: prose skill outside the worktree, `hydra/out-of-scope`. Listing with no path, `hydra/no-path`. A word count the floor could not parse, `hydra/floor`.

`CLAUDE.md` still says five config files. `SECURITY.md` lines 68 to 69 and harness spec 15.1 say six, including `config/panels.json`. The shell sentence still matches the gate.

## Stale and unproven

Lines 316 to 317 say six classes. `src/export.rs` `redact_secrets`, lines 625 to 678, has twelve labels, and AWS and Slack are among them. Lines 322 to 323 are stale on those two and right that JWTs are unnamed. A space-split or URL-encoded secret misses both checks, `scan_text` lines 795 to 798, with no test, so that half is unproven. `src/record/secrets.rs` lines 11 to 17 still flag five classes. That split is issue 108.

Line 10 says a shell key is absent. `src/env.rs` lines 28 to 35 copy proxy values whole. `IMPLEMENTATION.md` line 2784 says a proxy can carry a password. No `tests/` file plants one.

No test means unproven. Gate tests sit in `tests/gate.rs` unless a path is given.

## Holes the tickets already name

| Hole | In SECURITY.md |
|---|---|
| Issue 99, gated copilot writes `~/.claude.json`, user-scope MCP for the next ungated `claude` | No. Lines 72 to 74 say session state, not the file |
| Issue 108, three secret lists, cockpit shows keys export would redact | No. Lines 317 and 322 are the stale count |
| Issue 110, `jev ask` sends diffs to Typesafe, off the export allowlist | No |
| Issue 112, a run that can rewrite the local endpoint picks its model | No. Not built |
| Issue 114, `attack_succeeded` counts `completed`, and A3 and A5 have no defence | No |
| Issue 115, neither rung runs unless a human types it | No |
| Issue 131 (withheld), `standard` grants `shell` whole | Line 18 is the general fact. The rule is not stated |
| Build 130, interactive hooks on the other three providers unmeasured | No. Lines 65 to 67 are `claude` and plain `--interactive` |
| Build 318, Linux is hook-only | Yes. Lines 44, 67 and 100 |
| Codex hook dies open, builds 101 and 102 | Yes. Lines 277 to 296. `tests/drift.rs` `a_codex_hook_that_dies_without_answering_lets_the_call_run`, not run |
| Unset marker walks through, build 57 | Yes. Lines 227 to 228. `tests/delegation.rs` `a_launch_with_no_marker_is_not_refused_by_the_containment_check` |
| `ps -E` reads the human's terminals, builds 97 and 102 | Yes. Line 14. Watcher half tested by `tests/detached.rs` `the_watcher_holds_the_childs_environment_and_nothing_the_launcher_carried`. No foreign-terminal test |
| Admission test reads `~/.ssh`, issue 66 | Yes. Lines 98 to 99. No such test |
| Case fold, build 60 | Yes. Lines 45 to 49. `tests/spawn.rs` `a_layout_rooted_under_a_non_ascii_name_is_refused_before_writing`. Parent `.git` has no launch test |

## Holes the documents do not name

`src/policy/gate.py` `protected_path_named_in`, lines 457 to 469, does not resolve, so a relative MCP path walks through. `IMPLEMENTATION.md` lines 253 to 259 call that door closed. `SECURITY.md` lines 21 to 24 limit the residual to a shell token. Input: `mcp__writer__put` under `permissive` with a `..` walk into `runs/`. The absolute form is refused by `an_mcp_call_naming_a_protected_path_is_refused_under_permissive`. `a_protected_path_split_or_assembled_still_walks_past` allows MCP `//` and a split across fields, and never passes a relative path. The missing test expects `hydra/protected-path` on that call.

## Before going public

Research 129's flip list.

1. `gitleaks` 8.30.1 on 2026-10-15, 56 fixture hits, rerun still required. Not rerun here.
2. CI green. Not verified. `.github/workflows/security.yml` lines 28 to 35 grant `security-events: write`. `Cargo.toml` line 26 pins `sha2` 0.10.
3. README rewrite claimed, not audited. 4. `CHANGELOG.md` has `0.3.0` of 2026-10-15. Tag not checked. 5. Topics and preview not checked. 6. `config/README.md` line 10 says the allowlist is per machine. Values not opened. 7. Commit email left as the research file advises.
8. `pendintes_hydra.md`. Done. `.gitignore` lines 38 to 41.
9. `git gc`. Still waiting. Not done.

`SECURITY.md` has no flip list. Rows marked yes above are still open. Lines 329 to 336 say read the vendor terms. Not verified. The nine open tickets, several grillings, are what "resolve all of them" still needs.

## Appendix

| Claim | Where | Verdict |
|---|---|---|
| Allowlist, pty cleared | `src/env.rs` 123, `src/pty.rs` 120, `the_child_gets_the_environment_it_is_handed_and_nothing_else` | verified |
| Shell key absent, proxies copied | `src/env.rs` 28 to 35, `drops_the_secrets_this_shell_exports` | stale |
| Watcher uses child env | `src/run/mod.rs` 412, `the_watcher_holds_the_childs_environment_and_nothing_the_launcher_carried` | verified |
| `ps -E` reads human terminals | no code, no test | unproven |
| Spelled protected path refused | `gate.py` 357, `shell_is_refused_when_it_names_a_protected_path` | verified |
| Variable, subshell, relative, nested sibling allowed | `gate.py` 398, `the_shell_refusal_is_coarse_on_purpose` | verified |
| Quote, concat, percent, `..`, `//`, `/./`, MCP split allowed | `gate.py` 406, `a_protected_path_split_or_assembled_still_walks_past` | verified |
| Unparseable shell refused by floor | `gate.py` 430, `an_unparseable_command_is_refused_rather_than_guessed_at` | verified |
| Floor escape lifts floor only | `the_human_flag_lifts_the_floor_and_nothing_else` | verified |
| Bounded flags, redirect, `$`, glob, brace, newline, `sed` | `a_bounded_shell_refuses_what_its_list_does_not_name_and_says_which_part` | verified |
| Pipe and `&&` split | `a_bounded_shell_runs_the_read_only_commands_its_list_names` | verified |
| Symlink outside `file_read` refused | `a_bounded_shell_reads_where_file_read_reads_and_nowhere_else` | verified |
| Bounded profile text | `src/sandbox.rs` 402, `the_profile_for_a_bounded_shell_denies_worktree_writes_and_hydra_reads` | verified |
| Write outside `~/hydra` if unresolved | `SECURITY.md` 41 to 44, no live test | unproven |
| Non-ASCII root refused | `src/paths.rs` 75, `a_layout_rooted_under_a_non_ascii_name_is_refused_before_writing` | verified |
| Parent `.git` not checked | `src/paths.rs` 70, no test | unproven |
| Corpus, pruned, other spawns denied | `src/sandbox.rs` 107, `the_seatbelt_refuses_the_corpus_however_the_path_is_spelled` | verified |
| `TIOCSTI` denied | `src/sandbox.rs` 21, `the_seatbelt_refuses_terminal_input_injection` | verified |
| Launchd does not inherit | allow default, no test | unproven |
| Gated interactive wrapped, plain is not | `a_gated_interactive_launch_wraps_claude_in_the_seatbelt` | verified for `claude` |
| Linux unwrapped | `wraps_launches` 47, no Linux run | unproven |
| Checkout denied, worktree carved out | `the_checkout_is_denied_and_the_own_worktree_still_writes` | verified |
| `standard` shell reads the rest of the machine | no home deny, no `~/.ssh` test | unproven |
| Interactive home deny, session allows | `copilot_block` 317, `the_copilot_profile_is_the_headless_one_with_the_home_deny_and_its_allows_after_tiocsti` | verified |
| Own `claude` projects read | `own_read_paths` 1076, `a_claude_run_may_read_its_own_tool_results` | verified |
| Admission env cleared | `src/env.rs` 45, `run_admission_step_clears_the_parent_environment` | verified, library |
| Hostile test reads `~/.ssh` | not denied, no test | unproven |
| Private git index writable | `the_seatbelt_allows_git_add_in_the_worktrees_private_git_directory` | verified |
| Null git config, excludes passed back | `src/worktree.rs` 111, `a_file_ignored_only_by_a_global_excludes_file_is_neither_seeded_nor_committed` | verified |
| Both worktree launches refused | `a_launch_from_under_worktrees_is_refused`, `a_launch_from_under_skill_worktrees_is_refused` | verified |
| Own store, strict pack, 1 GiB | `src/worktree.rs` 395, `reap_moves_the_runs_objects_and_a_forged_pack_reaches_neither_the_parent_nor_a_later_worktree` | verified |
| Other git drops object dirs | `src/worktree.rs` 1467, library assert | verified |
| Unhooked Cursor tools | `src/policy/table.rs` 209, `unhooked_cursor_rows_are_known_and_carry_no_path` | verified, library. `UnhookedDenied` has no `tests/` name |
| Slack refused. Three policies lack network | `policies/standard.json` 9, `the_floor_refuses_slack_on_a_fetch_url_and_not_in_a_search_query` | verified |
| MCP descriptions not recorded | no negative test | unproven |
| Forged show boundary escaped | `a_fake_closing_marker_in_the_run_text_is_escaped_inside_the_real_one` | verified. Plain `error` and `hydra wait` unmatched |
| Hook writes refused, skill shell allowed | `a_run_cannot_rewrite_its_own_hook_or_provider_config_under_any_policy`, `a_shell_command_naming_the_skill_root_is_how_two_providers_read_a_skill` | verified |
| No stdin-to-shell tools | `src/policy/table.rs` 230, `a_shell_facility_carrying_no_command_is_denied_under_every_policy` | verified |
| Missing parent is containment | `src/containment.rs` 110, `a_launch_with_no_marker_is_not_refused_by_the_containment_check` | verified |
| `HYDRA_ROOT` refused in seatbelt | `src/run/mod.rs` 293, `tests/spawn.rs` near 126 | verified as a string |
| In-seatbelt delegate refused | `src/normalize/mod.rs` 1208, `a_non_interactive_runs_face_still_refuses_inside_the_seatbelt` | verified |
| Seat caps | `a_session_past_its_request_cap_is_refused_and_the_record_says_so_once`, `a_request_line_over_64_kb_is_refused_before_the_record` | verified |
| Lookalike MCP server refused | `a_server_whose_name_only_starts_with_a_listed_one_is_refused` | verified |
| Approval store denied | `tests/spawn.rs` 2036, `the_seatbelt_denies_a_write_to_the_approval_store` | verified. Live skip not re-measured |
| Bad payload denies. Codex fail-open separate | `evaluate_raise_is_answered_as_a_deny`, drift test not run | verified for the gate |
| Forged refusal kept, damaged line counted | `src/watch.rs` 339, `a_row_the_run_appends_to_its_own_sidecar_is_read_back_as_hook_evidence` | verified |
| Walk prints `Uncorroborated`, exits 0 | `src/walk.rs` 936, `tests/doctor.rs` near 423 | verified |
| Export abort, entropy warn, inode out | `src/export.rs` 623, `widened_secret_classes_abort_the_verifier` | verified. Six classes is stale |
| Space or URL-encoded secret missed | `src/export.rs` 795, no test | unproven |

## Pending from me

Suite, paid Codex probe, live seatbelt, `gitleaks`, Actions, tags, and allowlist values not run or read. Parent `.git`, proxy password, relative MCP path, and `~/.ssh` from admission not executed.

## Changed

Wrote `.scratch/hydra/research/133-seat-grok-security-audit.md` only. Nothing committed.

## Found

`SECURITY.md` states the holes it names, and is stale on the redactor: AWS and Slack are named, the record sweep flags five. Nine open tickets remain. Five closed exposures are still live. A relative MCP path is allowed because `protected_path_named_in` does not resolve, and no security document says so. The ignore rule, the `0.3.0` changelog section, and the workflow permission are in the tree. The scan rerun, green CI, the tag, and `git gc` are not shown from here.
