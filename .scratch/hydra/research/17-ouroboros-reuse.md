# 17 - What reusing razzant/ouroboros would actually cost

Status: findings only. Ticket 17 is a grilling ticket and the decision is the human's.

## What I did, and what I did not check

I cloned `github.com/razzant/ouroboros` into a temp directory outside this repo and read the
tree at `main`, tip `e9dfb359963789516eed4a3960c2cfaea95044b0`, dated 2026-08-30. Every number
below comes from that tree or from its git history.

Two things I could not confirm:

- Ticket 06 records a push on 2026-09-01. The tip of `main` is dated 2026-08-30 and the newest
  tag, `v6.113.4`, is also 2026-08-30. The 09-01 push landed somewhere other than `main`, or on
  a tag I do not see. Unverified either way, and it does not change anything.
- I did not install the dependencies, did not run the test suite, and made no model call. Claims
  about runtime behaviour are read off the code, not observed. Where that matters I say so.

I did import the concept modules with a bare Python 3.14 and no third-party packages installed,
which is a real check and is reported below.

## 1. Size and shape

It is an application, not a library. `pyproject.toml` declares two console scripts,
`ouroboros = ouroboros.cli:main` and `ouroboros-web = server:main`, and the repo also carries
`launcher.py` (65 KB), `Ouroboros.spec` for PyInstaller, `build.sh` / `build_linux.sh` /
`build_windows.ps1`, a `packaging/` tree, `Dockerfile`, and a `web/` UI of 127 files. There is no
stability statement anywhere. `grep -i "public api|stable api|semver|semantic version"` over
`README.md`, `CONTRIBUTING.md` and `docs/DESIGN.md` returns nothing. Nothing in the repo promises
you an interface.

1,380 tracked files. The checkout is about 41 MB of working tree plus 61 MB of `.git`.

| Directory | Lines | Files |
|---|---|---|
| `tests/` | 313,601 | 575 |
| `ouroboros/` | 214,429 | 339 |
| `web/` | 59,282 | 127 |
| `devtools/` | 47,795 | 151 |
| `supervisor/` | 27,191 | 27 |
| `docs/` | 18,532 | 35 |
| `assets/` | 14,729 | 16 |
| root files | 11,257 | 24 |
| `skills/` | 9,532 | 18 |
| `scripts/` | 5,361 | 21 |
| everything else | ~7,800 | ~65 |

Inside `ouroboros/`: 222 modules at the package top level (133,988 lines), `tools/` 67 modules
(54,807), `gateway/` 28 (18,251), `marketplace/` 9 (4,045), `contracts/` 11 (2,419).

Python total across the repo is 608,552 lines in 1,063 files, of which the tests are more than
half. The single biggest module is `ouroboros/loop.py` at 6,996 lines, then `ouroboros/llm.py`
at 4,414 and `supervisor/events.py` at 4,476. `server.py` at the root is 143 KB.

A structural quirk worth knowing before you read any of it. `ouroboros/review.py` sets
`MAX_MODULE_LINES = 1600` and `MAX_TOTAL_FUNCTIONS = 9500`, enforced by a `size_ratchet` pytest
lane that blocks in the project's own CI. 29 tracked Python files sit between 1,550 and 1,600
lines, right against the ceiling. The module count is high partly because the cap forces splits,
not because the design wanted 338 files in one package. Reading it means chasing a call across
three files that would be one file anywhere else.

Dependencies. 25 direct runtime requirements in `pyproject.toml`, resolving to 88 pinned packages
in `requirements-runtime.lock` and 115 in `uv.lock`. The list: `openai`, `gigachat`, `requests`,
`pip`, `httpx`, `jsonschema`, `ddgs`, `dulwich`, `huggingface_hub`, `starlette`,
`python-multipart`, `Pillow`, `pypdf`, `uvicorn[standard]`, `websockets`, `pytest`,
`pytest-xdist`, `pytest-timeout`, `mcp`, `PyYAML`, `croniter`, `tree-sitter`,
`tree-sitter-language-pack`, `tzdata`, `typing_extensions`. Five optional extras on top
(`browser`, `desktop`, `local-model`, `build`, `all`), which pull Playwright, pywebview and
llama-cpp-python.

Two of those are not what you would expect. `pytest`, `pytest-xdist` and `pytest-timeout` are
declared runtime dependencies, with a comment explaining why: the commit gate runs the test suite
itself and hard-blocks with `PREFLIGHT_PLUGIN_MISSING` rather than degrading to a serial run. And
`pip` is a runtime dependency because the self-update path shells out to `sys.executable -m pip`.
That is an agent that installs into its own interpreter at runtime.

Python floor is `requires-python = ">=3.10"`, ruff targets `py310`, classifiers stop at 3.12.
`tool.uv` pins `required-version = "==0.12.1"`, an exact uv version, which will break the day
someone's uv updates.

Licence is MIT, confirmed at `LICENSE`. No vendored third-party source that I found. The one
thing to note is `ouroboros/betterleaks_runtime.py`, which downloads a pinned external secret
scanner binary (`BETTERLEAKS_VERSION = "1.8.1"`) from a GitHub release at install time. That
binary's licence is not this repo's licence and I did not check it.

## 2. The three concepts, and whether one can be lifted alone

Short answer: the worktree module can. Baseline patch integration mostly can, if you accept
rewriting about two thirds of the file. The review panel cannot.

I measured coupling two ways, because the two answers differ a lot.

**Static import closure.** Following only module-level imports, `ouroboros.subagent_worktrees`
reaches 11 local modules and 6,349 lines. `ouroboros.tools.subagent_integration` reaches 35
modules and 29,471 lines. `ouroboros.tools.review` reaches 74 modules and 52,032 lines.

Following function-local imports as well, all four seeds I tried reach exactly the same 327
modules and 228,207 lines. So do `ouroboros.triad_review` and `ouroboros.config` on their own.
The deferred imports are how they break cycles, and the result is one strongly connected
component covering the whole package. The static number is meaningless past the top level, but
the fact it produces is real: there is no acyclic seam anywhere in `ouroboros/`.

**Actual import.** I put the repo on `sys.path` with no third-party packages installed and
imported each module. All succeeded, which surprised me and is a genuine point in reuse's favour:
the heavy dependencies are all lazy.

| Module | Imports cleanly on bare stdlib | `ouroboros.*` modules loaded |
|---|---|---|
| `ouroboros/subagent_worktrees.py` | yes, 0.09 s | 12 |
| `ouroboros/triad_review.py` | yes | 4 |
| `ouroboros/reviewer_slot_config.py` | yes | 4 |
| `ouroboros/tools/review_helpers.py` | yes | 39 |
| `ouroboros/tools/subagent_integration.py` | yes | 45 |
| `ouroboros/tools/review.py` | yes | 83 |

### Worktree sandboxing

`ouroboros/subagent_worktrees.py`, 1,000 lines, the only file in the repo with "worktree" in its
name. Its module-level imports are `platform_layer` (file locking), `utils` (atomic JSON write),
`config` (three path getters) and `retention` (a GC cutoff). Five `git` subprocess calls. It
loads 12 modules and no third-party code.

This one is genuinely liftable. The docstring states the design in full, the registry is a JSON
file at `data/state/subagent_worktrees.json`, and the cross-process lock exists because
`git worktree add/remove/prune` mutate shared `.git/worktrees` metadata. Everything ticket 06
recorded about it is still true at this commit.

The catch is that "liftable" and "stable" are different questions. It was 417 lines on 2026-07-05
and is 1,000 now, and the additions are not cosmetic: `provision_execution_snapshot`,
`find_execution_snapshot`, `prune_execution_snapshots`, `remove_execution_snapshot`,
`isolated_git_env`, `payload_capture_git_env`, `stage_raw_payload_inventory` and a new
`ExecutionSnapshotHandle` class all appeared in those eight weeks. You would be lifting a moving
target.

### Baseline patch integration

`ouroboros/tools/subagent_integration.py`, 1,599 lines, one line under the size cap. 29
functions. 11 `git` subprocess calls.

The mechanism ticket 06 praised is intact and is small. `_baseline_drifted_paths` at lines
311-355 is 45 lines: seed a scratch index from the baseline tree with `git read-tree`, stage the
current worktree state of exactly the touched paths, ask `git diff-index --cached` which entries
differ, and return an error string rather than a verdict when the comparison cannot be made. Its
docstring states the reason plainly, that a plain `git apply` relocates hunks by offset so the
apply's exit code cannot tell you whether the target moved.

Order of checks in `_integrate_subagent_patch` (line 625): locate patch, lineage check
(`INTEGRATE_LINEAGE_FORBIDDEN` when the caller has no `task_id` or the child's parent is someone
else), genesis-project refusal, sha256 of the artifact against the manifest, target resolution,
then the protected-path gate computed from `git apply --numstat` on the patch itself, then drift
proof, then apply.

One correction to ticket 06. It records the protected paths as four entries. The real policy
lives in `ouroboros/runtime_mode_policy.py` and is 24 paths in three named sets, plus one prefix:
`SAFETY_CRITICAL_PATHS` (`BIBLE.md`, `ouroboros/safety.py`, `ouroboros/runtime_mode_policy.py`,
`ouroboros/tools/extension_dispatch.py`, `ouroboros/tools/registry.py`, `prompts/SAFETY.md`),
`FROZEN_CONTRACT_PATHS` (`tests/test_contracts.py`, `docs/CHECKLISTS.md`,
`ouroboros/gateway/contracts.py`, `ouroboros/size_ratchet_manifest.py`) with the prefix
`ouroboros/contracts/`, and `RELEASE_INVARIANT_PATHS` (nine build and release files plus
`supervisor/git_ops.py`, `supervisor/update_merge.py`, `supervisor/update_merge_policy.py`). The
four-item list ticket 06 quoted survives only as an example in a docstring at line 360. Matching
is case-insensitive on purpose, because `write_file(path="bible.md")` on macOS or Windows writes
`BIBLE.md` and the uppercase membership test would have missed it.

How much of the file is mechanism and how much is Ouroboros policy. My estimate, by function
ranges: roughly 500 lines are the mechanism (`_patch_touched_paths`, `_stageable_paths`,
`_baseline_drifted_paths`, `_target_is_system_repo`, `_integrate_subagent_patch`, `_drift_refusal`,
`_locked_apply`, `_sha256_file`). The other ~1,100 are delegated-run custody, external-workspace
sharing, disposition recording, verdict files and tool registration, all of which encode
Ouroboros's task tree and would be dead weight here. That split is my reading, not a measurement.

It grew from 693 lines to 1,599 in the same eight weeks.

### The diff-review panel

This one is not a piece. It is 47 modules with `review`, `checklist`, `quorum` or `triad` in the
path, totalling 32,650 lines in `ouroboros/`, and that count excludes `config.py`,
`preflight_runner.py`, `capability_evidence.py` and the whole provider layer that they call.
Ticket 06's "~60 review-related modules" is the right order of magnitude.

`ouroboros/tools/review.py` loads 83 `ouroboros` modules on import. The largest pieces around it:
`tools/claude_advisory_review.py` 2,279, `review_state.py` 2,172, `skill_review.py` 1,600,
`tools/scope_review.py` 1,597, `review_substrate.py` 1,586, `tools/review_helpers.py` 1,575,
`review_evidence.py` 1,559, `tools/review.py` 1,550, `review_execution.py` 1,427,
`review_custody.py` 1,278, `reviewer_slot_config.py` 1,127.

Worse for extraction than the module count: the prompt is built from Ouroboros's own repo. At
`ouroboros/tools/review.py:1220-1221` the triad prompt loads the full text of `docs/DEVELOPMENT.md`
and `docs/ARCHITECTURE.md` from `ctx.repo_dir`, whole files, not sections. Those are 229 KB and
638 KB. `ouroboros/tools/review_helpers.py:37` sets `REVIEW_PROMPT_TOKEN_BUDGET = 920_000`.
`ouroboros/reviewer_window.py:12` puts the governance pack at "~169K tokens"; by a bytes/4
estimate the stable prefix is closer to 225K tokens per reviewer per round. There is also a small
inconsistency: `DEVELOPMENT.md` and `ARCHITECTURE.md` come from `ctx.repo_dir`, but
`review_helpers.load_checklist_section` reads `REPO_ROOT / "docs" / "CHECKLISTS.md"` where
`REPO_ROOT` is derived from the module's own file location. Reviewing a foreign repo still reads
Ouroboros's checklist.

Taking the panel means taking the governance documents it reads, or rewriting them. There is no
third option in this code.

## 3. The prompt assets

Mixed. The checklist is data on disk. The rules around it are hardcoded Python strings.

### The 29-item checklist

`docs/CHECKLISTS.md`, 843 lines, 98 KB, holding four checklists. The one ticket 17 means is
`## Repo Commit Checklist` at line 153. It is loaded at review time by
`review_helpers.load_checklist_section(section_name)` (line 895), which does a literal
`text.find("## " + name)` and slices to the next `\n## `. `ouroboros/tools/review.py:604` wraps
it fail-closed: a missing or malformed file raises rather than reviewing without a checklist.

So it is data, and it is the kind of data you can copy. It is also 37 KB for that one section,
which is too long to quote here. The 29 item names and their table severities:

| # | item | severity |
|---|---|---|
| 1 | `bible_compliance` | critical |
| 2 | `development_compliance` | critical |
| 3 | `secrets_check` | critical |
| 4 | `code_quality` | critical |
| 5 | `security_issues` | critical |
| 6 | `tests_affected` | critical |
| 7 | `architecture_doc` | critical |
| 8 | `version_bump` | critical |
| 9 | `changelog_and_badge` | critical |
| 10 | `tool_registration` | critical |
| 11 | `context_building` | advisory |
| 12 | `knowledge_index` | advisory |
| 13 | `self_consistency` | critical |
| 14 | `light_external_artifacts` | critical |
| 15 | `cross_platform` | critical |
| 16 | `changelog_accuracy` | advisory |
| 17 | `gateway_parity` | advisory |
| 18 | `subagent_isolation` | critical |
| 19 | `evolution_durability` | critical |
| 20 | `context_budget_ssot` | critical |
| 21 | `capability_regression` | advisory |
| 22 | `cache_friendliness` | advisory |
| 23 | `delegated_transport` | critical |
| 24 | `perf_lifecycle` | advisory |
| 25 | `source_completeness` | critical when applicable |
| 26 | `actor_readable_projection` | critical when applicable |
| 27 | `canonical_memory_fork` | critical when applicable |
| 28 | `review_artifact_continuity` | critical when applicable |
| 29 | `display_identity_replay` | advisory when applicable |

Ticket 06's account of the severity classes is right as far as it goes and incomplete in two
places. The `### Severity rules` block (line 198) also carves out item 13, conditionally critical
only when the mismatch falls in a named whitelist and a concrete stale artifact is cited, and item
21, advisory by default but escalating to critical for a silent removal of a documented capability.

The ugly part is the item bodies. The "what to check" cells are wildly uneven:

- item 2 `development_compliance`: 3,132 characters, and it is entirely about Ouroboros's own
  conventions, including the 1600-line gate, `MAX_TOTAL_FUNCTIONS`, and the exact name of the
  `size_ratchet` pytest lane
- item 18 `subagent_isolation`: 2,084 characters
- item 23 `delegated_transport`: 2,051 characters
- item 21 `capability_regression`: 11,100 characters in a single markdown table cell
- item 1 `bible_compliance`: 47 characters, because it delegates to a 46 KB constitution

Counting generously, about 7 of the 29 items are project-independent: `secrets_check`,
`code_quality`, `security_issues`, `tests_affected`, `cross_platform`, `self_consistency`,
`changelog_accuracy`. The rest name Ouroboros files, Ouroboros invariants or Ouroboros subsystems
(`gateway_parity`, `context_budget_ssot`, `canonical_memory_fork`, `tool_registration`,
`evolution_durability`). Taking the checklist means taking its shape and the severity machinery,
then writing 22 new items. That is still worth something. It is not 29 free items.

### The artifact-naming rule

This is the piece ticket 06 wanted to steal, and it exists twice.

In the checklist file, `docs/CHECKLISTS.md` lines 233-255, `### Critical threshold rule (applies
to ALL items)`, quoted in full:

> Before marking any item CRITICAL you MUST be able to answer YES to ALL of:
> 1. I can name the **exact file, symbol, function, test, or config path** in this
>    repository that makes this problem live RIGHT NOW.
> 2. That artifact actually appears in the diff or touched-file context I have been given
>    (not just in a hypothetical future scenario or external environment).
> 3. The fix requires a **change to this diff** - not a follow-up task or speculative guard.
>
> If you cannot satisfy all three, use **advisory**, not critical.
>
> For any finding about narrative, prose, or cross-surface consistency, also apply
> the `Critical surface whitelist` below (same rules for every reviewer - triad,
> scope, and advisory). A mismatch outside the whitelist is advisory.
>
> One root cause = one FAIL entry. Do NOT split one underlying problem into multiple
> FAIL items that all require the same change. Do NOT hold an obligation open by
> reformulating a fixed concrete issue into a broader future-risk variant - if the
> named artifact is fixed, mark PASS; raise a new advisory if a broader concern remains.
> Coverage is semantic, not numerical: zero or one FAIL is valid, and reviewers
> must never invent findings to reach a count.

(The three dashes in that quote are the file's own em dashes, replaced.)

And as a hardcoded Python string, `CRITICAL_FINDING_CALIBRATION` at
`ouroboros/tools/review_helpers.py:383`, injected into the triad, scope and advisory prompts
alike. It is 27 lines and says the same thing in seven numbered rules rather than three, adding
that hypothetical plugins and future integrations are advisory, that pre-existing gaps outside the
touched area are advisory, and that narrative mismatches are advisory unless they affect release
metadata, runtime behaviour or safety guidance.

Two more hardcoded strings in the same file are worth the same look:

- `REPO_ANTI_PATTERN_LOCK_GUARD` (line 432), a second-pass instruction that names concrete pairs
  of concern classes to cross-check, with an explicit "this is a semantic breadth check, not a
  numeric finding quota"
- `_ANTI_THRASHING_RULE_VERDICT` and `_ANTI_THRASHING_RULE_ITEM_NAME` (lines 445 and 452), which
  say the JSON `verdict` field is authoritative and withdrawal notes in prose are ignored, and
  that raising the same root cause under a new item name creates a phantom obligation
- `_CONVERGENCE_RULE_TEXT` (line 465): from attempt 3 onward, no new critical findings on code
  that did not change between attempts

Verdict on this section: the checklist is data and portable in shape only. The rule text is code,
about 120 lines of prompt constants across `review_helpers.py` and `triad_review.py`, and it is
the highest-value-per-line thing in the repository. Copying it is an afternoon. It is also the one
part where MIT attribution is the entire cost.

## 4. The self-edit gate

Ticket 06's two mechanical claims hold. Its framing of the gate as a blocker does not, because of
the default.

**Union veto: confirmed.** `ouroboros/tools/review.py:1029`:

```python
target = structured_critical if item.get("severity") == "critical" else structured_advisory
```

Every FAIL item from every responding reviewer with `severity == "critical"` lands in
`structured_critical`, and at line 1497 a non-empty `critical_fails` returns
`_build_critical_block_message`. One reviewer of three is enough. Nobody votes.

**Responsiveness quorum: confirmed.** `ouroboros/config.py:615-621`, unchanged from what ticket 06
recorded:

```python
return 2 if n_slots >= 3 else max(1, n_slots)
```

`ParsedTriadReview.quorum_met` at `ouroboros/triad_review.py:86` compares responding models
against `actor_records`, which holds every dispatched reviewer, so three configured with one
responding is a quorum failure. The call site at `review.py:1461` blocks separately with
`block_reason = "review_quorum"`. Defaults still `google/gemini-3.7-flash`,
`openai/gpt-5.6-terra`, `anthropic/claude-opus-5` for the triad and `openai/gpt-5.6-terra` for
scope, at `ouroboros/provider_models.py:313`. Slot limits still `TRIAD_SLOT_LIMIT = 10` and
`SCOPE_SLOT_LIMIT = 4`.

**The 63.5% block rate: not in the code.** `grep` for `block_rate`, `blocked_rate` and "block
rate" across all `.py` and `.md` returns nothing. That figure comes from the paper's Table 4 and
describes one deployment. The repository does not compute it, so it is not reproducible from this
tree and it is not a property of the software. Treat it as a report about "Hope", not a spec.

**Three things ticket 06 did not record, and they change the picture.**

First, `ouroboros/config.py:208` sets `"OUROBOROS_REVIEW_ENFORCEMENT": "advisory"` as the shipped
default, and `ouroboros/tools/review.py:1174` reads `blocking_review = review_enforcement ==
"blocking"`. Out of the box, critical findings do not block. They write a
`review_advisory_override` event (`review.py:947`) and a warning, and the commit proceeds. The
onboarding UI keeps that default too: `web/modules/onboarding_agents_step.js:173` falls back to
`'advisory'`, and `web/modules/settings.js:49` lists `advisory` as the settings default. Blocking
is opt-in. The benchmark configurations under `devtools/benchmarks/` are where you find
`"OUROBOROS_REVIEW_ENFORCEMENT": "blocking"` set explicitly. So the 63.5% number describes a
deployment that had turned the gate on; the software as shipped does not have it on.

Second, the severity a finding carries is whatever the reviewer model wrote in its JSON.
`ouroboros/triad_review.py:398`:

```python
"severity": str(entry.get("severity") or "advisory").lower(),
```

There is no table in code mapping item 11 to advisory or item 3 to critical. The checklist's
severity column and the whole `Severity rules` section are enforced only by the prompt. A model
that returns `{"item": "context_building", "verdict": "FAIL", "severity": "critical"}` blocks the
commit, even though the checklist says item 11 cannot block. The item name is free text and is
never validated against the 29. This is exactly the ticket 05 failure mode, in the reference
implementation, in the gate that is supposed to be the last line of defence.

That is not an oversight, it is doctrine. `BIBLE.md` Principle 5, "LLM-First", line 477: "All
decisions, responses, routing, planning through the LLM. Code is the minimal transport between the
LLM and the external world." The review prompt's own preamble at `review.py:37` restates it as "If
behavior can be described in a prompt, it goes in a prompt, not in code." Ouroboros's constitution
mandates the thing ticket 05 concluded does not work. If Hydra reuses this gate, it inherits that
position, not just that code.

Third, and in the other direction, the part of the gate that is deterministic is large and it is
the interesting part. `ouroboros/preflight_runner.py`, 1,488 lines, "Hermetic pytest preflight for
reviewed repository changes", assembles a candidate tree and runs the whole test suite in two
passes (parallel with `-n auto --dist loadscope --max-worker-restart=0 --timeout=300`, then a
serial pass for real-subprocess suites) before any paid review runs. `ouroboros/commit_admission.py`
calls this "ADMISSION policy": the checks that decide whether a candidate may spend review budget
at all. The suite is 575 files and 9,430 test functions.

That is the whole D8 argument sitting in the reference implementation as 1,488 lines of
compensating machinery. Ouroboros has no compiler, so it built one out of pytest, and the shape of
the file (plugin verification, `addopts` counter-tampering, per-test timeouts converting hangs into
named failures) is what that costs. `cargo build && cargo test` is the same admission gate for
free, minus the tampering surface.

Enforcement bypasses I looked for and did not find: `ouroboros/owner_hurry.py` relaxes plan review
to advisory when the owner clicks "hurry up", and its docstring is explicit that "Commit
triad/scope, advisory pre-review, deterministic P3 gates, and safety NEVER consult hurry". I
believe it, having read the module, but I did not test it.

## 5. Release cadence, and what pinning would mean

Commits on `main` per ISO week:

| Week | Commits |
|---|---|
| 2026-W25 | 63 |
| 2026-W26 | 38 |
| 2026-W27 | 39 |
| 2026-W28 | 16 |
| 2026-W29 | 27 |
| 2026-W30 | 33 |
| 2026-W31 | 51 |
| 2026-W32 | 301 |
| 2026-W33 | 146 |
| 2026-W34 | 332 |
| 2026-W35 | 418 |

1,324 commits in the eight weeks since 2026-07-06, and the rate is accelerating, not settling.
807 of those (61%) are authored by "Ouroboros" itself, 423 by Anton Razzhigaev, the rest spread
across seven other names. 75 tags in the same eight weeks, so roughly nine releases a week.
`VERSION` went from `6.56.0` on 2026-07-05 to `6.113.4` on 2026-08-30. That is 57 minor bumps in
eight weeks, which tells you the version number is a build counter and carries no compatibility
meaning.

Churn in the files you would want:

| File | Commits (8w) | Diff | 2026-07-05 | Now |
|---|---|---|---|---|
| `ouroboros/config.py` | 75 | +973/-885 | | 1,598 |
| `docs/CHECKLISTS.md` | 62 | +305/-237 | 773 | 843 |
| `prompts/SYSTEM.md` | 47 | +416/-209 | | 901 |
| `ouroboros/tools/review.py` | 32 | +765/-504 | 1,273 | 1,550 |
| `ouroboros/reviewer_slot_config.py` | 21 | +1341/-214 | | 1,127 |
| `ouroboros/triad_review.py` | 13 | +401/-27 | 283 | 657 |
| `ouroboros/tools/subagent_integration.py` | 8 | +990/-84 | 693 | 1,599 |
| `ouroboros/subagent_worktrees.py` | 6 | +638/-55 | 417 | 1,000 |

The two integration modules more than doubled. The checklist went from 21 items to 29.

Has the interface changed? There is no declared interface, so I diffed top-level `def` and `class`
names in the four concept modules between 2026-07-05 and now:

- `subagent_worktrees.py`: 12 names added, none removed, one new class. Additive.
- `subagent_integration.py`: 20 names added, none removed. Additive.
- `triad_review.py`: `_empty_array_is_verified_clean` became public as
  `empty_array_is_verified_clean`; `extract_fenced_json`, `is_provider_oversize_error`,
  `parse_review_findings`, `review_query_error_payload` added.
- `tools/review.py`: `_format_timeout_seconds`, `_parse_model_response`,
  `_review_model_timeout_sec` and `_review_query_error_payload` removed; seven added, including
  `_dispatch_unified_review` and `_prepare_unified_review`, so the dispatch path was reshaped.

Across the whole package: 130 of the 338 current `ouroboros/*.py` files (38%) were added in the
last eight weeks, one was deleted, and zero were renamed. The growth is by accretion, which the
1600-line cap makes inevitable.

What that means for the three options. Tracking upstream is not viable: 165 commits a week into a
228k-line cyclic package with no interface promise, driven substantially by an agent, is a
full-time merge job. Vendoring a snapshot is the only honest form of "reuse" here, and a snapshot
of a project accreting 38% new modules per two months is a fork on day one whether you call it
that or not. Forking and diverging is the same thing said out loud. The one clean case is copying
the prompt text, which is data, carries no upgrade path, and is the piece that changes slowly:
`CHECKLISTS.md` grew by 70 net lines over eight weeks while `subagent_integration.py` grew by 906.

## 6. What a Rust reimplementation would have to rebuild

Mechanical, and I would not budget much for any of it:

- The worktree registry. JSON file, cross-process file lock, orphan prune on startup, five `git`
  subprocess calls. `subagent_worktrees.py` is 1,000 lines because Python; the logic is small.
- The drift proof. 45 lines. `git read-tree` into a scratch index via `GIT_INDEX_FILE`,
  `git update-index --stdin`, `git diff-index --cached`. Identical in Rust, whether you shell out
  or use git2.
- `git apply --3way --index`, and the separate non-3way working-tree path.
- Protected-path matching. Three frozen sets, one prefix, a lowercase compare.
- `adaptive_quorum`. One line.
- The union veto. One conditional.
- The checklist and every prompt constant. Data.

Genuinely hard, in rough order of how much I would expect it to hurt:

1. **Tolerant parsing of reviewer output, and telling a refusal from a clean pass.**
   `empty_array_is_verified_clean` at `triad_review.py:311` exists because `[]` buried in refusal
   prose ("I cannot review this diff... []") was entering the quorum as an approval. The fix is a
   whole-response match with one optional code fence and an optional `NO_FINDINGS` sentinel. This
   is thirty lines you will not think to write until it bites you, and when it bites you the
   failure is a self-edit that was never reviewed being recorded as reviewed.
2. **The obligation ledger across retries.** `review_state.py` (2,172), `review_custody.py`
   (1,278), `review_evidence.py` (1,559). Findings carry an `obligation_id`; blocked attempts
   write structured obligations the next round must address by name; `loop.py:1296`
   `_reopen_obligation_row` reopens one when it recurs; repeated blockers synthesise
   "commit-readiness debt" that keeps the blocking path shut until cleared. The anti-thrashing
   prompt rules only work because this state exists underneath them. Rebuilding the rules without
   the ledger gets you the appearance of the defence.
3. **Reviewer context-window resolution as evidence rather than a table.**
   `ouroboros/reviewer_window.py` plus `capability_evidence.py` (1,001 lines). The docstring
   records two production failures worth reading before designing this: a hardcoded
   `context_window=1_000_000` sized a 200K route as 1M and drew a provider 400, losing that
   reviewer's entire review; and a process-lifetime memo that outlived a 24h evidence TTL made a
   healthy install read its own scope reviewer as expired forever, blocking every commit for the
   rest of the process's life. Both are the class of bug you only find by shipping. The design
   answer they landed on, that sizing may be optimistic but blocking authority may not, is worth
   copying regardless of language.
4. **Fitting the review prompt.** They inject whole governance documents and budget 920K tokens
   per reviewer. Hydra will need its own answer, and "the diff plus a checklist" is probably it,
   but the machinery for deciding what to drop when a diff is large (the "fit note", the path
   manifest fallback, the binary-file representation) is real work.
5. **Reviewer routing.** `reviewer_slot_config.py` (1,127 lines, +1341/-214 in eight weeks) exists
   because a reviewer row can be an API model or another agent harness, each with its own effort
   level, and because slot ids must be stable across reordering so receipts keep lining up. Hydra
   already routes four CLIs, so some of this lands on ground the router owns. It is not free.

What Rust gets for free that Python paid 1,488 lines for: the admission gate. `preflight_runner.py`
is a hermetic test-suite runner with plugin verification and `addopts` tamper-checking, standing in
for the compile step. That is the strongest evidence I found for D8, and it comes from the
reference implementation's own code rather than from an argument about it.

What Rust does not get for free, and this cuts the other way: nothing in a type system stops a
reviewer model from returning `"severity": "critical"` on an advisory item. Rust makes the
severity table enforceable, which Ouroboros chose not to do. It does not make the reviewer honest.

## The two-line version

The prompt assets are worth taking and cost an afternoon. The worktree module and the drift proof
are worth reading closely and are small enough to rewrite from the reading. The review panel is
32,650 lines welded to another project's constitution, defaults to not blocking, and takes its
severities on faith from the model. I would not reuse the code.
