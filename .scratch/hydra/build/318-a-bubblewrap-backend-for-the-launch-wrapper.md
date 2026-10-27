# 318 - A bubblewrap backend composed from the same `Spawn` as the seatbelt profile

Type: build
Status: resolved
Blocked by: 315, 316
Opened: 2026-10-15, by the session, from map 119's answer, question 2. On Linux Hydra launches
with the hook config and no OS sandbox. The panel chose bubblewrap over Landlock because the
seatbelt profile `compose()` writes is allow by default, deny the parent, allow the child back,
last match wins, and bwrap's mount order has the same last-wins rule, while Landlock only grants
and cannot spell "deny `~/hydra/runs` under an allowed `HOME`".

## What

`src/sandbox.rs` composes the seatbelt profile from a `Spawn` and wraps the launch in
`sandbox-exec`. Add a Linux backend behind `cfg(target_os = "linux")` that composes a `bwrap`
argv from the same `Spawn`, rule by rule in the same order as `compose()`, so that the two
backends are one table with two renderings. The twelve rule kinds of the profile translate to
`--ro-bind`, `--bind`, `--tmpfs` and `--dev-bind` mounts; where one does not translate, the
backend refuses to launch with a message naming the rule, never a silent omission.

Three things the panel said the build must settle, in the Outcome and in code:

- `wraps_launches()` turns true on Linux only when `bwrap` is on PATH and user namespaces are
  enabled (`/proc/sys/kernel/unprivileged_userns_clone` or the distro's equivalent); otherwise
  the launch refuses under `standard` and `permissive` rather than falling back to hook-only.
- Codex: under a wrapper Hydra hands `codex` `danger-full-access` because a second sandbox
  cannot nest in the seatbelt (`src/provider/codex.rs` lines 82 to 86). Decide whether codex's
  own sandbox nests inside bwrap (it probably does not: both want user namespaces) and make the
  argv follow from that decision, not from `wraps_launches()` alone.
- `inside_seatbelt()` is false on every non-macOS target and five human-only commands key on it
  (`src/stop.rs` line 73, `src/approve.rs` lines 85 and 147, `src/face.rs` line 374,
  `src/run/mod.rs` line 295, `evolve.rs` line 2009). The Linux witness is that a mount namespace
  cannot be shed: compare the launcher's `/proc/self/ns/mnt` with the one `spawn_config.rs` records
  at launch, or an equivalent you can measure.

The `sandbox` object of build 316 gets `kind: "bubblewrap"` with the sha of the argv.

## Do

1. Read `src/sandbox.rs` whole, `docs/harness-spec.md` sections 7 and 15.2 (row 4), and the
   map 119 memos under `.scratch/hydra/research/119-seat-*.md` and `119-synthesis.md`.
2. The composer, as a pure function from `Spawn` to argv, with unit tests on macOS that check the
   argv for each rule kind and the refusal for the untranslatable ones.
3. The gating of `wraps_launches()` on Linux, the codex decision, the `inside_seatbelt()` witness.
4. `cargo check --target x86_64-unknown-linux-gnu` clean; the macOS suite green. The runtime test
   on a Linux machine is the human's and is not part of this ticket; say so in the Outcome.

## Done means

The pure composer tested, the three decisions written in the Outcome, the cross check clean,
`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green. Amend
`IMPLEMENTATION.md` and `docs/harness-spec.md` row 4 where the seatbelt is named as the only
wrapper.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

Built in two tickets: build ticket 318a in run `01a17db0-8922-7c30-a20a-f0ab131d945e`, `claude` with `claude-haiku-5-5` under `standard`, 3,248,593 tokens, wrote the composer; build ticket 318b in run `01a17db9-eaf0-7e33-93c7-c9311ee859d6`, `claude` with `claude-haiku-5-5` under `standard`, 8,403,409 tokens, wired it in and settled the three decisions. The full record is in each ticket's Outcome.

The shared rule table: `rules_of` returns the sixteen rule kinds in the order `compose` wrote
them. `render_sbpl` turns them into the seatbelt profile, and `compose_bwrap` turns them into
bwrap arguments. `compose_bwrap` skips the plugin manifest regex, because its concrete paths
already come through the plugin write denial. `translate_rule_to_bwrap` refuses the other
regex-bound rules by name, the interactive TIOCSTI rule without `legacy_tiocsti` and the
copilot's regex allow. `rules_of` never emits those two. The seatbelt text is unchanged.

The three decisions:

- `wraps_launches()` on Linux is true only when `bwrap` is on `PATH` and unprivileged user
  namespaces are open. `standard` and `permissive` refuse without that. `judge` runs bare.
- Codex runs with `danger-full-access` under either wrapper. Neither `sandbox-exec` nor bwrap
  lets Codex's `workspace-write` user-namespace sandbox nest inside Hydra's wrapper. bwrap marks
  the parent mounts locked, and the worktree's gitdir and object store sit outside the worktree.
- `inside_seatbelt()` on Linux compares the process's mount namespace with the `launcher.mnt`
  that the launcher writes beside the profile. A witness that names another namespace means
  the process runs inside bwrap.

Verification, on macOS and for the Linux target: `cargo check --target x86_64-unknown-linux-gnu`,
`cargo clippy --release --workspace --all-targets -- -D warnings` on macOS and on Linux, `cargo fmt
--all --check`, and `git diff --check` are clean. `cargo nextest run --release --workspace` passes
1494 tests with 4 skipped, and `cargo test --release --workspace --doc` reports no failures.

The runtime test on a Linux machine is the human's and is not part of this ticket. No Linux
host has run the bwrap argv yet.
