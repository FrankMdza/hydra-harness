# Build 318a - Pure bubblewrap composer from `Spawn` sharing one rule table with `compose()`

Status: claimed
Size: haiku

## Goal

Refactor `src/sandbox.rs` so that the macOS seatbelt composer (`compose`) and the Linux bubblewrap composer (`compose_bwrap`) are driven by one shared rule table (`rules_of(set, spawn) -> Vec<Rule>`) in the exact same order, with `translate_rule_to_bwrap` refusing untranslatable rules by name rather than silently omitting them.

## Exact edits

1. **In `src/sandbox.rs`:**
   - Define `pub enum Rule` representing each rule kind in `compose()` plus the untranslatable regex rule kinds:
     1. `TtyTiocstiDeny` (rule 1: TIOCSTI ioctl deny on headless PTY runs)
     2. `InteractiveTtyTiocstiDeny { legacy_tiocsti_disabled: bool }` (rule 1 on interactive TUI runs where `--new-session` cannot be used without detaching the controlling terminal)
     3. `WriteDenySubpath(String)` (rule 2: write denies from `write_denies(set, spawn)`)
     4. `ReadDenySubpath(String)` (rule 3: read denies on `corpus`, `spawns`, `pruned`, `approvals`)
     5. `UnixSocketConnectDeny { spawns_dir: String, allow_seat_socket: Option<String> }` (rule 4: unix-socket connect deny under `spawns/` with own `seats.sock` allowed)
     6. `BoundedReadDataDeny(String)` (rule 5: `file-read-data` denies under a bounded shell)
     7. `ReadAllowSubpath { path: String, bounded: bool }` (rule 6: read allows on `own_spawn` and `bounded_read_allows`)
     8. `ReadMetadataAllowLiteral(String)` (rule 7: metadata-only allow on `spawns/` when `parent_git_dir.is_some()`)
     9. `WriteAllowSubpath(String)` and `WriteAllowLiteral(String)` (rule 8: write allows on `own_write_paths`, `gate-refusals.jsonl`, and `own_read_paths`)
     10. `GitAdminWriteDeny(String)` (rule 9: write denies on worktree private `HEAD`, `commondir`, `gitdir`)
     11. `CodexSkillsWriteDeny(String)` (rule 10: write deny on `codex` copied skills root)
     12. `PluginWriteDenySubpath(String)` and `PluginManifestRegexDeny { component: String, suffix: &'static str }` (rule 11: plugin subpath write denies and the two manifest directory regexes)
     13. `BoundedWorktreeWriteDeny(String)` (rule 12: final worktree write deny under a bounded shell)
     14. `CopilotWriteAllowRegex(String)` (the `~/.claude.json` and `~/.claude/history.jsonl` regex prefix rules from `copilot_block`)
   - Implement `pub fn rules_of(set: &PolicySet, spawn: &Spawn<'_>) -> Vec<Rule>` producing the exact sequence of rules that `compose(set, spawn)` emits today.
   - Implement `pub fn render_sbpl(rules: &[Rule]) -> String` and make `compose(set, spawn)` call `render_sbpl(&rules_of(set, spawn))`. The SBPL string produced by `compose(set, spawn)` MUST be byte-for-byte identical to what `compose(set, spawn)` produced before this ticket.
   - Implement `pub fn translate_rule_to_bwrap(rule: &Rule) -> std::result::Result<Vec<String>, String>`:
     - `Rule::TtyTiocstiDeny` -> `Ok(vec!["--new-session".to_string()])`
     - `Rule::InteractiveTtyTiocstiDeny { legacy_tiocsti_disabled: true }` -> `Ok(Vec::new())`
     - `Rule::InteractiveTtyTiocstiDeny { legacy_tiocsti_disabled: false }` -> `Err("rule 1 (interactive TIOCSTI ioctl deny) has no bubblewrap mount equivalent when dev.tty.legacy_tiocsti is not 0".to_string())`
     - `Rule::WriteDenySubpath(p)` | `Rule::GitAdminWriteDeny(p)` | `Rule::CodexSkillsWriteDeny(p)` | `Rule::PluginWriteDenySubpath(p)` | `Rule::BoundedWorktreeWriteDeny(p)` -> `Ok(vec!["--ro-bind".to_string(), p.clone(), p.clone()])`
     - `Rule::ReadDenySubpath(p)` | `Rule::BoundedReadDataDeny(p)` -> `Ok(vec!["--tmpfs".to_string(), p.clone()])`
     - `Rule::UnixSocketConnectDeny { spawns_dir, allow_seat_socket }` -> `let mut out = vec!["--tmpfs".to_string(), spawns_dir.clone()]; if let Some(sock) = allow_seat_socket { out.extend(["--bind".to_string(), sock.clone(), sock.clone()]); } Ok(out)`
     - `Rule::ReadAllowSubpath { path, .. }` -> `Ok(vec!["--ro-bind".to_string(), path.clone(), path.clone()])`
     - `Rule::ReadMetadataAllowLiteral(p)` -> `Ok(vec!["--dir".to_string(), p.clone()])`
     - `Rule::WriteAllowSubpath(p)` | `Rule::WriteAllowLiteral(p)` -> `Ok(vec!["--bind".to_string(), p.clone(), p.clone()])`
     - `Rule::PluginManifestRegexDeny { component, suffix }` -> `Err(format!("rule 11 (plugin manifest regex /{component}{suffix}) has no bubblewrap mount equivalent"))`
     - `Rule::CopilotWriteAllowRegex(pattern)` -> `Err(format!("copilot regex rule (^{pattern}) has no bubblewrap mount equivalent"))`
   - Implement `pub fn compose_bwrap(set: &PolicySet, spawn: &Spawn<'_>) -> Vec<String>`:
     - Starts with `vec!["--dev-bind".to_string(), "/".to_string(), "/".to_string(), "--proc".to_string(), "/proc".to_string(), "--die-with-parent".to_string()]`.
     - Iterates over `rules_of(set, spawn)`: skips `Rule::PluginManifestRegexDeny` (whose concrete worktree and home paths are already covered by `Rule::PluginWriteDenySubpath`) and appends `translate_rule_to_bwrap(&rule).expect("translatable spawn rule")` for every other rule in order.

2. **In `src/sandbox.rs` (`mod tests`):**
   - Add unit tests:
     - `compose_and_compose_bwrap_share_one_rule_table_across_policies_and_providers`: checks that `render_sbpl(&rules_of(&set, &spawn))` equals `compose(&set, &spawn)` across providers and policies, and that `compose_bwrap(&set, &spawn)` begins with `--dev-bind / / --proc /proc --die-with-parent --new-session`.
     - `compose_bwrap_renders_each_rule_kind_in_compose_order`: checks that for a `Spawn` with `parent_git_dir` on `Provider::Codex` and on a bounded-shell policy, `compose_bwrap` emits `--ro-bind` for write denies (kind 2), `--tmpfs` for read denies (kind 3) and socket parent (kind 4), `--ro-bind` for own spawn (kind 6), `--dir` for `spawns` metadata (kind 7), `--bind` for own writes and `gate-refusals.jsonl` (kind 8), `--ro-bind` for `HEAD`, `commondir`, `gitdir` after the own write allow (kind 9), `--ro-bind` for `.codex/skills` after the worktree allow (kind 10), `--ro-bind` for plugin write paths after the worktree allow (kind 11), and under bounded shell `--tmpfs` for bounded read denies (kind 5) and `--ro-bind` for bounded worktree write deny at the end (kind 12).
     - `translate_rule_to_bwrap_refuses_untranslatable_rules_naming_each_rule`: checks that `translate_rule_to_bwrap` returns `Err` naming the rule for `Rule::PluginManifestRegexDeny`, `Rule::CopilotWriteAllowRegex`, and `Rule::InteractiveTtyTiocstiDeny { legacy_tiocsti_disabled: false }`.

## Verification

Run:
- `cargo fmt --all --check`
- `cargo clippy --release --workspace --all-targets -- -D warnings`
- `cargo nextest run --release -E 'test(sandbox)'`

## Done criteria

- Only `src/sandbox.rs` and `.scratch/hydra/build/318a-compose-bwrap-from-spawn.md` are modified.
- All three verification commands exit 0.
