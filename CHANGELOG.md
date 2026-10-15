# Changelog

Dates are when the change landed on `master`. Run ids are the evidence.

## Unreleased

## 0.3.0, 2026-10-15

- 2026-10-15, later. Map 130 sat a four-seat panel on every decision still open and the human took
  the session's recommendations: build 316 goes additive (`host` and `sandbox_kind`, schema version
  stays 1), reap refuses a nested repository by name (326), the refusal sidecar stays unsigned with a
  corroboration check in the walk (332), the three refusal facts stay in text (320), maps 126 and
  127 answered. The `linux` CI job's first run was red on 51 tests in eight groups; build 331 fixes
  four groups on a real Linux, decides two, and skips two by name. `security` workflow repaired
  (`security-events: write`), three stray files ignored, `gitleaks` over the whole history clean.

- 2026-10-15. Six grillings closed with the human on the panels' recommendations (maps 117 to
  123) and thirteen builds opened from them. Landed the same day: Hydra compiles on Linux, with a
  `/proc` start-time reader and an `ubuntu-latest` CI job (build 315, run 01a13dff-46f9); the
  drift table is keyed by provider, OS and arch and `hydra doctor` says unmeasured where a platform
  has no row (317, 01a13dff-478b); the pattern floor splits commands on a real newline, reads a
  quoted heredoc body fed to `cat` or `tee` as data, and exempts single quotes from the
  substitution rule only (319, 01a13e0a-87ca); a plan proposer that never produced a draft is
  labelled `chosen: null` rather than rejected (323, 01a13e0e-fe45); the live judge panel can write
  a `composed` verdict because its prompt now asks for the task array (322, 01a13e0e-c767). Map
  124 read 932 records for what each provider is good for; map 125 measured GPT-OSS 120B against
  Gemini 3.8 Flash and kept Flash in the `agy` seat; map 129 is what stands between the repository
  and a public one. The README was rewritten to a third of its length, with the install, roadmap
  and authorship moved under `docs/`.

- Map ticket 64 closed with the human: the delegate face is not available under the seatbelt, and
  since build ticket 76 a `delegate` call from inside a macOS run is refused as
  `hydra/seatbelt-delegation` before any corpus read, so the parent's record carries Hydra's words
  and not the kernel's. The map is 67 of 67 (2026-09-22).
- The three map tickets the panel opened closed with the human the same day and five of the six
  build tickets landed, each on a Composer 2.5 run reviewed by the session: the whole checkout
  is denied to writes under every policy (map 65, build 74); rung two's admission gate runs
  under the proposer's own seatbelt with an empty `HYDRA_ROOT` and the pass record names the
  profile (map 66, build 75); the seatbelt is defence behind the gate, and the spec says so (map
  67); a run can `git add` inside the seatbelt and still cannot move a branch (build 70); `hydra
  resume` works on `codex`, measured (build 71); the pty tee has a deadline after the child
  exits (build 72). The gated interactive launch runs under the seatbelt (build 73) and waits on
  a human measurement (2026-09-22).
- A hiring-panel review by seven readers, four of them the providers through `hydra run`,
  found 33 things and 22 are fixed: the gate answers a deny for a malformed payload instead of
  dying, `HYDRA_ROOT` is refused inside a seatbelt, a sibling's spawn directory is read-denied,
  a PEM block is redacted to its footer and both export files are verified before either is
  created, the harvest holds a lock, the face reaps its watchers, the two git parsers read `-z`,
  four more commands take a prefix, `hydra resume` on `codex` is refused with the reason, and
  `hydra doctor` exits 0 on a healthy install. The README's numbers are the corpus's again.
  `docs/reviews/2026-09-22-seven-readers-as-a-hiring-panel.md` has the table (2026-09-22).
- Measured versions are the 2026-09-22 reading: `claude` 2.1.268, `cursor-agent`
  2026.09.18-9a7762b, `agy` 1.2.8, `codex` 0.154.0, and the founding measurement holds on all
  four (2026-09-22).
- `hydra evolve --task` gives the proposer a target; the first directed pass produced the
  first patch a panel reviewed as a patch (2026-09-09).
- `hydra show` and `hydra reap` take a run-id prefix (2026-09-10).
- Admission refuses patches to the code rows harness spec 15.2 names, not only `policies/`
  and `config/` (2026-09-12, build ticket 40).
- A curate pass no longer tries to delete the proposal branch it just made (2026-09-12,
  build ticket 43).
- A run can no longer rewrite the hook or MCP config Hydra wrote inside its worktree, on any
  provider, under any policy; the skill roots it links are denied to writes and readable by
  shell (2026-09-12, build ticket 44).
- Several runs launching on one skill-store sha no longer race to create the snapshot
  (2026-09-12, build ticket 47).
- The rate-limit abort reads the field the vendor writes and fires (build ticket 41); a curate
  run's record says `command: curate` (build ticket 42).
- A `Write` Claude Code refuses before reading the file is no longer read as ungated
  (2026-09-12).
- The replay no longer calls an `agy` command a gate leak when the stream cut its argument at
  513 characters; it records a limit and the hook's own allow (2026-09-12, build ticket 45).
- Two parallel calls to one tool on `claude` each get their own hook answer; the first used to
  read as ungated (2026-09-12, build ticket 46).
- README rewritten around the measurement Hydra exists for; `llms.txt` added; demo GIF
  recorded from a real `agy` run (2026-09-12).
- `codex` is the fourth provider, under `standard` and `permissive`: the gate rides a `-c`
  override naming a hook file in the spawn directory, because Codex reads no project hook
  file inside a git worktree (2026-09-21, build ticket 48, run 01a0c231-6583).
- A proposer rate-limited by its last frame ends an `evolve` pass `completed` on a blocked
  branch, the way one rate-limited by outcome already did (2026-09-20).
- The floor's `git` parser consumes the value after `-c` and `--config-env`, so `git -c
  core.pager=cat push --force` is refused the way `git push --force` was (2026-09-21, build
  ticket 58, a change to protected config #1 made by the human).
- `codex` sits on the `evolve` panel, four judges now; `read-only` stays refused on it
  (2026-09-21, build ticket 61, map ticket 63).
- The self-improvement module is `src/rungs/` and its spec `docs/rungs-spec.md`; new passes
  write `rungs/<pass-id>` branches and the old `ouroboros/` ones still resolve (2026-09-21,
  build ticket 62).
- The tenth `evolve` pass is the first the panel offered: four judges, `codex` among them,
  all pass on a two-word patch, on `rungs/01a0c4b4-81ec`; the human merged it as 24cade6, the
  first change to land through rung two (2026-09-21).
- `hydra logs --follow` waits up to two seconds, not one poll, for the closing line after the
  pid reads dead, because a loaded machine can take longer than 100ms between the reap and the
  write (2026-09-21, seen once on CI).
- The judge sees the proposer's task as evidence that waives nothing, the checklist with its
  definitions, and the baseline of any test the patch edits (2026-09-21, build ticket 63, map
  tickets 53 and 57).
- `hydra evolve --review <pass-id> --evidence <file>` runs a fresh full-panel review of a blocked
  pass; a block is never overridden by hand (2026-09-21, build ticket 64).
- `preference.jsonl` has a writer, with judge verdicts as a second source behind
  `config/export-signoff.json`, which the human writes (2026-09-21, build ticket 65, map 55).
- The hook-config writer is `src/spawn_config.rs`, a protected row of harness spec 15.2
  (2026-09-21, build ticket 66, map 60).
- A launch from a checkout that is not the data root refuses and names `HYDRA_ROOT`; `hydra
  doctor` is the twelfth command (2026-09-21, build ticket 67, map 61).
- On macOS every recorded launch runs under the kernel's seatbelt: writes under Hydra's
  directories and reads of the corpus fail whatever the spelling, measured on all four providers;
  delegation from inside a run waits on map ticket 64 (2026-09-21, build ticket 68, map 62).
- The gate reads the paths out of an `apply_patch` body, so `codex` writes under `standard`
  (2026-09-21, build ticket 69).

## 0.2.0, 2026-09-08

All eleven commands of harness spec 17. Three routed providers, five policies, the run record
contract, the skill store with per-run snapshots, the MCP delegate face, the Herdr bridge,
both Rungs rungs. `IMPLEMENTATION.md` carries every departure from the design.
