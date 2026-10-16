# 133 panel, opus seat: the architect's reading

I read the files the task names plus build 348, with grep and reads. No tests, no launches, no refusals.

## What I verified

- `gate.py::record_refusal` writes a row only when `capability_of(tool) == "shell"` and the command is a string, `src/policy/gate.py:1806-1807`. Build 339's premise holds.
- `protected_path_named_in` resolves nothing, and its docstring says a relative MCP path walks through, `src/policy/gate.py:457-469`. Build 348's premise holds.
- `CODEX` holds five rows, `WebSearch` unhooked and no `webrun`, `src/policy/table.rs:395-401`.
- `attempt.rs` sets temperature 0, `crates/apprentice/src/attempt.rs:89`, and keys attempts on `(task_id, prompt_sha, model)`, line 303.
- `hydra` depends on `hydra-puppets` as a library, `Cargo.toml` line 32, and uses it in `src/show.rs:31,142`. Every crate under `crates/` is `publish = false`.
- SECURITY.md lines 316 and 317 say six classes. That matches the plan.
- The plan says CLAUDE.md line 203 says five protected configs. In this worktree, line 203 says "six config files" and line 212 says "six protected configs". The claim is wrong here, or it came from another copy.
- The "house JSON equality" the runner would use is `hydra_pyjson::py::eq`, `crates/pyjson/src/py.rs:16-18`. Its doc says `True == 1 == 1.0`. `Value` derives no `PartialEq`, `json.rs:105`.
- Builds 340 to 347 have no files yet.
- IMPLEMENTATION.md:1063 admits that `git push` with no arguments goes unchecked. No build ticket owns it.

## The five questions

1. Must be fixed before going public: 348, 339, the bare `git push`, and map 99. The session puts 99 under fix after, and I disagree. A gated session that writes `~/.claude.json` plants a server that the next ungated session starts. That is the one exposure on the list that carries a run out of the gated population into the ungated one. Each of the others stays inside one run. This comes from the ticket's title and the plan's summary. I did not read 99's body against the code. 348 also matters because two documents call that door closed.
2. The format is one file too big. `README.md` repeats the ticket's objective, so drop it. Keep `contract.json` and `cases.jsonl`. More important, the equality is wrong for a typed contract. `py::eq` passes `true` where the case expects `1`. The grader's differential tests compare "equal to the byte", `crates/grader/tests/differential.rs:9`, and the runner should do the same: `json::dumps` both sides with sorted keys and compare the strings. On JSON Schema I would skip the `jsonschema` crate. `hydra-puppets` is in the published binary's dependency tree, so the schema crate's dependencies would ship in `hydra` too. A checker for `type`, `properties`, `required`, `items` and `enum` takes about a hundred lines. Latency should be printed and never fail CI, because "this Mac, M2 8 GB" is not the CI runner.
3. Covered in the next section.
4. Measure the router of MobileMoE-S on CPU through Transformers on the bank's tagged prompts first. It is 0.68 GB at Q4, by the gemini seat's table. Its licence bars shipping, not measuring. OLMoE in bf16 needs 14 GB by the same seat, and this machine has 8 GB with swap already near full, so 345 as written cannot run here. The cheapest result that kills the idea: per-domain routing histograms classify the bank's domains no better than build 294's dense embeddings plus a logistic head. If that happens, MoE gives nothing that map 109 does not already give. Set the threshold in 133 before the run.
5. I would not spend a week on it. Spend one day on one metric: whether a recorded denied call replays to the same decision. I did not check which projects have a hook layer, so I name none.

## The architect's role

The contract format fits functions that run on their own, like `expert_profile`. It does not fit 338, 339 or 348. Their input is a hook payload, and their runner already exists: `tests/gate.rs` compiles the gate and feeds it synthetic payloads. Their prose `## Contract` and `## Cases` are enough. The format's only real consumer is 345, which waits on map 133. Building the runner and `echo` before that is building before the need. Make 344 wait on 345's cases being written.

`hydra-puppets` is the wrong home for the runner. It is the goal log for Master of Puppets, `crates/puppets/src/lib.rs:1-2`, and `hydra` links it. A runner there either links into the core binary or needs a feature gate to stay out. A new `crates/contract` with one bin, depending on `hydra-pyjson` alone, keeps the core binary unchanged and costs one manifest.

Cuts and order:

- 339's case 2 uses `webrun` as the unknown tool. Once 338 lands, `webrun` is known and the case breaks. Use `webrun2`, which is 338's case 4 name. With that change, 339, 348 and 338 can land in any order.
- 338's probe is partly answered already. The gate refused `webrun` in run `01a14290-c0a1`, so the hook fired, so the row is hooked and not unhooked like `WebSearch`. Inferred from the plan; I did not read the record.
- 339 says "canonical JSON" without defining it. The gate is Python, so the ticket should name `json.dumps(sort_keys=True, separators=(",", ":"), ensure_ascii=False)`, or the sha will differ between a reader in Rust and the writer.
- Merge 347 into 344. A README and one CLAUDE.md paragraph are not a ticket.
- Land 341 before 340, so the new section is not written on top of stale text. 341 covers IMPLEMENTATION.md only. SECURITY.md's own stale lines, 316 and 317, have no owner.
- 340 says `Size: sonnet`. Section 4 defines haiku and opus only, and the session's own provider memory keeps building off Sonnet. Make it opus.
- 346 changes the attempt key at line 303 to add a sample index, and should say what happens to existing attempts files. Keep it, after 345.
- Drop 343 from this plan. The ponytail trial needs ten tickets with hidden tests that do not exist, and it answers a question none of the human's words ask. Give it a map ticket of its own.

Two tickets the plan should open:

1. A build ticket for the bare `git push`. The plan's map ticket 134, item 5, recommends fixing it before going public, and no build carries it. The hook can read the cwd's `HEAD` when argv names no refspec.
2. A drift check that keeps 338 from coming back: `hydra doctor` compares each installed provider's version with the version its table row was measured on, `CODEX_MCP_PREFIX`'s comment says 0.154.0 at `table.rs:406`, and warns on a mismatch. A seat found 0.160.0 by accident.

## What I could not verify

Map 99's mechanism, the twelve IMPLEMENTATION.md admissions by line, run `01a14290-c0a1`'s record, and any external figure in the seats.

## Pending from me

Nothing. The human decides on 99's class and on whether 343 stays.

## Changed

Wrote `.scratch/hydra/research/133-panel-opus.md`. Nothing else, nothing committed.

## Found

`py::eq` is Python's loose equality and wrong for contracts. `hydra` links `hydra-puppets`, so a runner there ships in the core binary. 339's case 2 breaks once 338 lands. 345 cannot run OLMoE on this machine. The bare `git push` has no ticket. The CLAUDE.md "five configs" claim does not hold in this worktree.
