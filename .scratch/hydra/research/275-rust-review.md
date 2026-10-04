Not confirmed

- **The corpus size.** I could not check the "~800 records" figure because `~/hydra/runs` is refused under this policy. Every cost below is counted per record, so the multipliers hold at any corpus size.
- **Per-call replay cost.** I took the claim that a gate replay costs one interpreter process per record from the doc comment at `src/policy/replay.rs:150` and the comments in `src/normalize/mod.rs:521`. I did not read the interpreter launch itself.
- **The cockpit mascot.** I did not confirm that it needs redraws on a timer. That only matters for finding 4.
- **Agent work never ran.** The `standard` policy refused every Agent call, so I read the whole workspace myself. `src/cockpit/chat.rs`, `src/cockpit/mascot.rs`, `src/skills/`, `src/mcp.rs`, `src/sandbox.rs`, `src/seats.rs` and most of `crates/grader` got sweeps (unwrap, casts, slicing, duplicate names, function length) and spot reads, not a line-by-line review. A finding I would have made there is missing from this report rather than wrong.
- **Compiler.** I ran no cargo command. No finding needed a compiler's word, and the ticket changes nothing.

## Findings, ranked by value over risk

### Efficiency

**1. `hydra doctor --walk` normalizes every record nine times.** `src/walk.rs:68-84`: `walk_paths` passes the same `read` closure to eight checks, and `check_normalizes` adds a ninth pass. Each call is `read_record`, which runs `normalize::read_with`, and that replays the gate. So every record pays nine full reads, nine parses and nine replays. `check_normalizing_writes_nothing` at `src/walk.rs:359-365` also holds every record's bytes in memory at once, which matters on an 8 GB machine.
- **Smallest fix:** memoize `read` inside `walk_corpus`. Keep a `HashMap<PathBuf, Result<Run, String>>` and return clones; `Run` already derives `Clone`. Run `check_normalizing_writes_nothing` first so its before/after snapshot still brackets the first read.

**2. `hydra export` normalizes the corpus once, plus once per allowlisted repo, plus twice more for every eligible run.**
- `audit` at `src/export.rs:114-127` normalizes every ended record and keeps the text and `Run` of all of them. Then it returns only the `Verdict`s.
- `read_judge_pairs` at `src/export_preference.rs:144-151` scans and normalizes the whole corpus again for every repo in the allowlist, even though the scan never uses `repo`.
- `collect_pairs` at `src/export_preference.rs:297` normalizes each eligible run again, and so does the export loop at `src/export.rs:339`.
- **Smallest fix:** normalize once in `export_with_signoff` and pass a `&BTreeMap<String, Run>` to `audit`, `collect_pairs` and `read_judge_pairs`. Ended records are immutable, so the output does not change.

**3. The cockpit reader re-reads every record on every 2 s tick, ended ones included.** `src/cockpit/snapshot.rs:658-664`. For each record, `live::read_view` opens the file twice: line 0 at `src/live.rs:469`, and a tail read of up to 64 KB at `src/live.rs:493`. Then `line_zero` at `src/cockpit/snapshot.rs:1005` opens it a third time, and `opening_of` parses the same line 0 a second time. The `record_info` cache only skips the task, model and `ended_at` lookups. An ended run never changes, yet it pays all of this every tick.
- `read_run_info` at `src/cockpit/snapshot.rs:547-565` also re-reads and re-folds every goal log under the puppets home on every tick.
- `count_live` at `src/cockpit/snapshot.rs:789` reads a live record whole each time it grows, so the total cost grows with the square of the run's length.
- **Smallest fix:**
  - Keep a `RunView` per run id, keyed by length.
  - Skip `read_view` once the state is `Ended` and the length has not moved.
  - Pass the line 0 that `read_view` already read out to `opening_of`.
  - Cache goal folds by length and mtime.

**4. The cockpit redraws unconditionally.** `src/cockpit/mod.rs:220-234` draws every pass of the loop: every 100 ms, or every 15 ms with a chat pane. Each frame rebuilds `detail_lines` (`src/cockpit/app.rs:3003`, 160 lines of work) and `goal_table_lines` (`:1245`). A dirty flag set by a snapshot, an event, chat output or a mascot tick would remove almost all of these frames. The risk is the mascot's timing, which I did not confirm.

**5. Two whole-file reads to get line 0.** Five functions read line 0.
- `reap::line_zero` at `src/reap.rs:221-235` and `resume::first_envelope_model` at `src/resume.rs:162-171` call `read_to_string` on the whole record to get one line.
- `resume::provider_session_id` at `src/resume.rs:142` reads the same file whole a second time.
- `live::first_line` at `src/live.rs:469` already reads only line 0. Making it `pub(crate)` and calling it from `reap`, `resume` and `cockpit::snapshot::line_zero` removes three readers.

**6. Repeated linear searches.** Minor at today's sizes; a `HashMap` of views by run id fixes both.
- `status::abandoned_worktrees` at `src/status.rs:311` runs `views.iter().find` once per worktree directory.
- `reap::one` at `src/reap.rs:241` does the same once per id.

**7. Smaller repeats.**
- `walk.rs:669` and `walk.rs:775` call `derive(&run)` a second time when `derived` from `:569` and `:755` is in scope. Use `derived.source` and `derived.outcome`; that also removes two `unwrap`s.
- `src/cockpit/snapshot.rs:893-896` clones `Ended`, including `result_text`, for every ended card on every tick.

### Correctness

**8. The cockpit's shell tokenizer reads bytes as characters.** `src/cockpit/call_head.rs:445` and `:498` do `bytes[i] as char`, and `shell_segments` pushes those into `word`. Any non-ASCII command turns into mojibake: `é` becomes `Ã©`. Worse, UTF-8 continuation bytes 0x85 and 0xA0 become U+0085 and U+00A0, which `is_whitespace` accepts, so `à` (C3 A0) splits a word in half. Display only, but it shows on any path with an accent.
- **Fix:** iterate `char_indices()`. This changes behaviour, so it is a fix of its own and not part of any refactor ticket.
- While there: the `ch == '|' && ...` test at `:481` is dead code, because `:470` already returned on `|`.

**9. A write-path secret class list that does not match export's.**
- `src/record/secrets.rs:11-17` flags five classes on the write path.
- `src/export.rs:578-591` redacts twelve, plus OpenAI keys.
- `crates/grader/src/redact.rs:16-29` repeats export's twelve.

The cockpit hides a line only when it carries `secret_flag` (`src/cockpit/snapshot.rs:1109`, `call_head.rs:569`). So an `sk-proj-`, `AKIA`, `xoxb-`, `gho_` or `github_pat_` key in a record shows in the cockpit tail even though export would redact it. `record::secrets::prefixes()` has no caller in `src/` or `tests/`, even though its doc says the cockpit masks by it. Widening the write-path sweep changes what a record carries, so **this is a decision for the map, not a refactor.**

**10. A scroll limit truncated to `u16`.** `src/cockpit/app.rs:1331` and `:1400` cast `total.saturating_sub(visible) as u16`, so past 65,535 wrapped rows `max_scroll` wraps to a small number. `:1341` and `:1410` then add `delta as u16` with plain `+`, which panics in a debug build near the top. The wheel at `:1063` uses `saturating_add` with no clamp to `max_scroll`.
- **Fix:** `u16::try_from(..).unwrap_or(u16::MAX)` plus `saturating_add`. This also changes edge behaviour, so it stays out of the tickets.

**11. Panics on input a user or provider controls.** I found none reachable. Every `unwrap`/`expect` outside tests guards an invariant established a few lines earlier: `normalize/*` replay counts, `run.rs:2153-2191`, `triage.rs:101-137` behind the `ends.is_empty()` return, `live.rs:356`, `capture_page_layer` at `app.rs:1145`. Three of them are `is_none()` checks followed by `unwrap()` and should be `let ... else`: `src/export_preference.rs:183-187`, `src/rungs/evolve.rs:94-95` and `src/cockpit/app.rs:1939-1943`.

**12. Error context flattened.** `src/run.rs:1331` turns an error into a string with `anyhow!("{err:#}")`, which only exists because `match &mounted` borrows. Write `let mounted = ...?;` instead. The message stays the same and the error chain survives.

### Duplication

**13. The four normalizers repeat their plumbing.**
- `results_by_call` exists five times, with identical bodies: `normalize/claude.rs:750` and `:1431`, `cursor.rs:767`, `agy.rs:788`, `codex.rs:889`.
- The prologue of each `decide` repeats: collect calls, build `ReplaySpawn::of`, `scope_limit`, `Replay::compile`, `decide_all`. See `claude.rs:482-541`, `cursor.rs:485-541`, `agy.rs:530-560` and `codex.rs:646-700`.
- Each `hydra_line` copies the same `run_start` fields: policy, policy_sha, store_sha, cwd, worktree, gate (`cursor.rs:251`, `agy.rs:205`, and their twins).
- `note_unknown` appears four times.
- The provider sniff is written three times at `normalize/mod.rs:499`, `:534` and `:556`.

The four `decide` functions are 228 to 280 lines each. Their middles really do differ by provider, so only the shared ends should move.

**14. Enum-to-string helpers copied between files.** `role_str` exists three times: `src/export.rs:635`, `src/export_preference.rs:445`, `src/show.rs:148`. `status_str` exists twice: `src/export.rs:642`, `src/show.rs:155`. `Decision` and `Decider` already have `as_str` at `normalize/mod.rs:175` and `:201`; `Role` and `ResultStatus` should get theirs. `export_event` at `src/export.rs:450-502` also repeats its exclusion list in an inner match that can never be reached; flattening it to one match removes about eight lines.

**15. Pass branch names and `branch_exists` written twice.** `src/export_preference.rs:333-353` and `src/rungs/evolve.rs:1331-1350` both list `rungs/blocked/`, `ouroboros/blocked/`, `rungs/` and `ouroboros/`, and both define `branch_exists`. That function returns `Result<bool>` but can never be `Err`, so the `?` at both call sites and the `unwrap_or(false)` are noise.

**16. Python's `str.splitlines()` written three times in the crates.** `crates/puppets/src/read.rs:211`, `crates/puppets/src/panel.rs:190` and `crates/grader/src/labels.rs:577`, two of them in one crate. Both crates depend on `hydra-pyjson`, so it belongs in `hydra_pyjson::py`. The byte splitters at `grader/src/records.rs:234` and `puppets/src/log.rs:154` have different semantics on purpose; leave them.

**17. Two folds of one goal log.** `src/cockpit/goal.rs:198` versus `crates/puppets/src/fold.rs:148`. Both are documented as deliberate, since `hydra` does not depend on the puppets crate. I list it for drift risk. Removing it is a layering decision.

**18. Git spawned outside `worktree.rs`.** The crates call git directly in `crates/puppets/src/panel.rs:1068`, `write.rs:137` and `crates/grader/src/jev.rs:107-140`. Each matches a different Python stderr behaviour, so merging them changes output. I am not proposing it.

### Structure

**19. `src/cockpit/app.rs` is 3,865 lines.** One `impl App` holds 103 lines of fields and every concern. Turn it into `app/mod.rs` with child modules. Child modules see the parent's private fields, so no visibility changes are needed:
- `text.rs`: `:3235-3357` and `:3514-3727`, the pure helpers, about 330 lines. Move this first, it carries the least risk.
- `confirm.rs`: the `draw_*_confirmation` functions and their texts, `:3358-3513`.
- `review.rs`: `ReviewRun` and `:1926-2139`.
- `stop.rs`: `StopRun` and `:1750-1911`.
- `pages.rs`: places, page layers and scrolling, `:1113-1472`.
- `draw.rs`: `:2140-3234`.

While moving the scroll code, merge `goal_table_content_metrics`/`scroll_goal_table_lines`/`goal_table_scroll_key` at `:1323-1381` with their copy at `:1392-1446`. Only the line source and the scroll field differ. Also remove `let _ = width;` at `:1319`.

**20. `src/normalize/claude.rs` has an inline `mod harvest` of about 480 lines.** It runs `:1134-1611` and is the interactive transcript reader. It should be its own file.

**21. `src/run.rs` holds about 2,200 lines of code before its tests.** The seams already exist:
- lineage and depth, `:171-365`
- the refusals, `:366-443`, `:623-731`, `:1028-1123` and `:1762-1810`
- the record writers, `:1811-2144`
- the resume argv, `:2145-2200`

**22. `src/main.rs:11` is a 263-line `main`.** It takes `provider` as a `String` and parses it by hand. A clap `value_parser` would reject a bad name with the valid list and remove the `.parse::<Provider>()?` calls.

### Idiom

**23. Provider and outcome as strings.** `RunView.provider`, `Run.provider` and `Card.provider` are `Option<String>`, and there are 32 `Some("claude" | ...)` matches outside tests (12 in `normalize/mod.rs`, 6 in `outcome.rs`). `Card.outcome: Option<String>` makes `lane_of` at `src/cockpit/snapshot.rs:86` match on `"completed"`. `Provider` and `Outcome` enums already exist. This is worth doing only together with ticket 4, which removes three of the matches anyway.

**24. Seven `run::Request` literals of 14 fields each.** In `seats.rs:742`, `:772`, `resume.rs:114`, `curate.rs:156` and others. A constructor with defaults would remove about 60 lines. Low value.

## Refactor tickets

None of these changes behaviour. Each can be made separately with the suite as its net.

1. **Walk each record once.** Memoize `read` in `walk::walk_corpus` and run `check_normalizing_writes_nothing` first (finding 1). Net: `tests/doctor.rs`, `tests/record_contract.rs`.
2. **Normalize the corpus once per export.** Build one `BTreeMap<String, Run>` in `export_with_signoff` and thread it through `audit`, `collect_pairs` and `read_judge_pairs` (finding 2). Net: `tests/export.rs`.
3. **Cockpit reader caches.** Skip `read_view` for an ended run whose length has not changed, read line 0 once per record, and cache goal folds by length and mtime (finding 3). Net: the cockpit and snapshot tests.
4. **Shared normalizer plumbing.** Move into `normalize/mod.rs`: a free `results_by_call(&[Event])`, one `provider_of(text)` for the three dispatchers, and one `RecordedStart` that each `Walk` holds and builds `Recorded` from. Leave the provider-specific middles of `decide` alone (finding 13). Net: `tests/normalize.rs`, `tests/outcome.rs` and the fixture walk.
5. **One home per small helper.** Add `Role::as_str` and `ResultStatus::as_str`, and flatten `export_event` to one match (finding 14). Write the pass branch names once in `rungs` and make `branch_exists` return `bool` (finding 15). Make `live::first_line` the one line-0 reader for `reap`, `resume` and the cockpit (finding 5). Move `splitlines` into `hydra_pyjson::py` (finding 16).
6. **Split `cockpit/app.rs`** into `app/{text,confirm,review,stop,pages,draw}.rs`, with `text.rs` first, and merge the two scroll blocks (finding 19).
7. **Split `run.rs` and `normalize/claude.rs`** into `run/{lineage,refuse,record}.rs` and `normalize/claude/harvest.rs` (findings 20 and 21).
8. **Idiom sweep:**
   - `let ... else` at the three `is_none`/`unwrap` sites
   - reuse `derived` at `walk.rs:669` and `:775`
   - remove the dead `|` branch at `call_head.rs:481`
   - remove `let _ = width;`
   - use `?` at `run.rs:1331`
   - a clap `value_parser` for `--provider`

   Each item is a few lines and keeps behaviour exactly.

Kept out of the tickets because each changes behaviour: finding 8, the UTF-8 tokenizer fix; finding 10, the `u16` scroll clamp; finding 4, the dirty-flag redraw; and finding 9, which belongs on the map as a decision.

## Run report

- **Changed:** nothing. I edited no file and ran no cargo command, so I have no fmt, clippy or test results to report.
- **Tests added:** none.
- **Out of date in the ticket:** it says `src/cockpit/app.rs` is 3,794 lines; it is 3,865 now. Nothing else it assumed turned out false.
- **Not mine:** the worktree already held uncommitted changes from someone else, including untracked `src/marker.rs` and `crates/pyjson/src/marker.rs`. I left them alone and did not review them.

## Pendiente de mí

- Finding 9 needs a decision on the map: should the write-path sweep in `record/secrets.rs` widen to export's classes, and should the dead `prefixes()` go?
- Finding 8 and finding 10 each need a small fix ticket of their own that is allowed to change behaviour.
- Decide whether tickets 4 and 6 are worth their size. They are the largest, and the suite covers them well.

## Cambiado

Nothing.

## Encontrado

- **Biggest costs:** the doctor walk at 9× normalization per record, the export at 1 + repos + 2 per eligible run, and the cockpit re-reading ended records every tick.
- **One real display bug:** `call_head.rs`, where bytes are read as characters.
- **One security mismatch for the map:** three secret class lists that disagree, with the cockpit following the shortest one.
- **Agents refused:** this policy refuses the Agent tool, so a review this size runs in a single context. Wider coverage of the files listed under Not confirmed needs a launch from outside the run.