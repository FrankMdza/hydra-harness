# 67 - Is the seatbelt a boundary or defence in depth, and which does the document claim?

Type: grilling
Status: open
Blocked by:

## Question

Map ticket 62 chose the macOS seatbelt over a second uid, and build ticket 68 wrote the profile:
`(allow default)`, fourteen `file-write*` denies, a `file-read*` deny on the corpus and, since
2026-09-22, on every other run's spawn directory. Nothing restricts `process-exec`, `mach-lookup`
or `mach-priv*`. A profile binds the process it wraps and that process's descendants. It does not
bind a process that launchd, LaunchServices or the Apple Event dispatcher starts on the run's
behalf, because that process is launchd's child and inherits launchd's context. One `launchctl
submit` probe on 2026-09-22 under a real profile did not produce the file; that is one spelling.
`SECURITY.md` said "whatever the spelling" about paths and nothing about leaving the tree, and
since 2026-09-22 it says the profile is defence behind the gate and not a boundary.

1. **Is that the claim?** If the seatbelt is defence in depth behind the gate, the profile stays as
   it is, the sentence stays, and the residual is a class the document names. Cheap, honest, and
   what the document says today.
2. **Or is it a boundary?** Then it needs `(deny process-exec)` outside an allowlist, denies on the
   mach services that reach launchd and LaunchServices, and a measurement per provider of what
   breaks: `claude` spawns MCP servers and its own shell, `cursor-agent` writes `cursor-data/`
   before the model runs, `codex` runs with its own sandbox off inside Hydra's. Each deny is a
   provider that may stop starting.
3. **`codex` is the case that decides.** Under the seatbelt it runs `--sandbox
   danger-full-access`, because its `workspace-write` could not nest inside Hydra's profile. So
   on macOS a `codex` run's writes are bounded by Hydra's profile, which allows everything outside
   Hydra's directories, where the vendor's default would have bounded them to the workspace.
   Hydra's wrap makes that provider's reach wider. Whether Hydra's profile should cover what
   `workspace-write` covered is the same question as 2 from the other side.

Until this closes, `SECURITY.md` scopes the seatbelt to the process tree, names the class that
leaves it, and states the `codex` comparison.
