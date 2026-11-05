# Panel 73, opus

## 1. Recommendation

A new option, 4. The human names the paths a pass may edit when launching it, and there is a fixed floor of paths no pass may touch even if named. Baseline tests are handled by build ticket 80's refusal, not by moving probes. I would not build option 1 as written, because its example set includes `src/provider/`, and `src/provider/claude.rs:162-163` is where the `--settings` flag that loads the gate goes onto the argv.

## 2. Mechanism

- `src/cli.rs:165-168`: `evolve` already takes `--task`. Add a repeatable `--may-edit <path>`. When it is absent, use a short default set. About 10 lines.
- `src/rungs/evolve.rs:469-492`: keep `protected_path_hit` as the floor. It is `PROTECTED_CODE_PATHS` at `448-465` plus the config rows, with `Cargo.toml`, `Cargo.lock`, `build.rs` and `.cargo/` added. Add `outside_allowed(paths, allowed)` beside it and call it right after the floor check at `166`. If a named path overlaps the floor, the pass refuses to start. The pass record carries the allowed list. About 40 lines.
- Build ticket 80: a patch that edits or deletes a file under `tests/` that exists at `base_tree` is refused. `changed_paths` at `evolve.rs:1143-1158` already lists the paths. Checking them against `git ls-tree -r --name-only <base_tree> tests/` is enough. New test files are allowed. The ticket estimates 60 to 100 lines.
- `tests/evolve.rs`: `222` asserts `src/lib.rs` passes the check, so it becomes a test that an unnamed path is refused and a named one admitted. Add one test for the baseline refusal on a synthetic patch. Both would fail today, because nothing refuses an unnamed path or an edited baseline test. About 60 lines.

Roughly 170 to 210 lines, all in already-protected files.

## 3. What it breaks or costs

Undirected passes, with no `--task`, are bounded by the default set. Branch tips also carry the seeded worktree, so I took each proposer's files from `IMPLEMENTATION.md:711-716` and `2998-3012` and build ticket 36 at `100-126`:

- Pass 2 touched the three normalizers. Option 1's example set admits it.
- Pass 5 touched `evolve.rs` and `IMPLEMENTATION.md`. Every option refuses it.
- Passes 7 and 8 touched `cli.rs`, `main.rs`, `reap.rs`, `show.rs` and a new `resolver.rs`, and pass 7 edited `tests/cleanup.rs`. Option 1 refuses both.
- Passes 9 and 10 touched `src/cli.rs` and `src/run.rs`. Pass 10 is `e0c782b`, the only rung-two patch ever merged. Options 1 and 2 both refuse it.

So option 1 admits one of six real patches and refuses the one that landed. Option 4 admits pass 10 once the human names its two files, and ticket 80 refuses pass 7's test edit at admission, which today only `agy`'s judge caught.

The cost is a flag per pass. A proposer that needs a file the human did not name gets refused, and the pass is spent.

## 4. What a hostile run or proposer still gains

A named file is fully editable. If the human names `src/run.rs` to fix help text, the proposer can change which policy a child gets in the same patch. The floor does not cover `run.rs`, and harness spec 15.2 says deliberately that it "stays editable by rung two". Only the panel and the human's merge stand between that patch and master. A unit-test probe inside a named file, like A12's in `src/face.rs`, can be weakened together with the code it guards, because ticket 80 covers only `tests/`. A new test file is allowed, so a proposer can add a test that passes whatever the code does. The judge sees it, and no mechanism does.

## 5. The strongest argument against your choice

It moves the security decision to launch time, when the human is thinking about the feature and not the gate. A static allowlist is reviewed once, in a protected file. `--may-edit src/run.rs` gets typed fast and hands over the launch sequence, and in pass 10 the task was two words of help text. Nothing warns the human that a named file is mechanism. The floor warns only for files someone remembered to list, which is the failure the ticket opens with.

## 6. The other options

**Option 1.** It is right that a new file starts refused. But its example set would have refused five of six real patches, and it includes the provider modules that carry the gate flag. Every widening becomes a protected-file edit for a two-line change.

**Option 2.** It fixes today's four files, and the next mechanism file repeats the miss, as `evolve.rs:453-458` already counts three times. Moved probes don't protect the ones left behind.

**Option 3.** Refusing all of `tests/` means a proposer cannot ship the test that proves its change. Ticket 80 refuses edits to existing tests and still allows new ones, which is the part that matters.

## 7. Something the ticket missed

Yes, there is a better option than the three listed. It is option 4 above: a per-pass list the human names, a floor, and build ticket 80.

Also missed:

- Option 1's example set is unsafe. A patch dropping `src/provider/claude.rs:162-163` launches `claude` without its hook config.
- Harness spec 15.2 says `run.rs` "stays editable by rung two". Protecting it reverses a written decision, not a miss.
- Nothing refuses a patch that creates `build.rs` or `.cargo/`, or adds a dependency with a build script. After merge the human's own `cargo build` runs it unsandboxed.
- Nothing refuses edits to `CLAUDE.md` or `IMPLEMENTATION.md`, which every later session reads as instructions.
