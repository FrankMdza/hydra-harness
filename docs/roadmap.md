# Roadmap

What is open, in the order it is likely to land. Each line names the ticket that carries it, so the
state is read from the ticket and not from this page.

- **Linux.** Build 315 makes Hydra compile there (a `/proc` reader behind `cfg(target_os)` and a
  CI job on `ubuntu-latest`); build 318 composes a bubblewrap profile from the same rules as the
  seatbelt, after 316 puts the host and the sandbox kind on every record. Until 318, a Linux
  launch is hook-only and recorded runs stay on macOS (map 119).
- **The floor reads shell the way bash does.** Build 319: a quoted heredoc body and single-quoted
  text are data, a newline separates commands. Build 320: a refusal names the deny entry, the
  token and the link it matched through. Build 321: a refused command lands whole on the record.
- **A panel that can compose a plan.** Build 322; then 323 and 324 give the labels a `null` for
  proposers without a draft and a row for the composed plan (map 122).
- **Codex under `read-only`.** Routed since build 48 under `standard` and `permissive`, on the
  judge panel since 61, writing through `apply_patch` since 69. It has no read tool, so map 63
  refuses `read-only` until a bounded shell read exists.
- **A gated interactive session on `codex`, `cursor-agent` and `agy`.** Build 130 is open, and
  `integrations/herdr/interactive-gate-providers.txt` lists `claude` alone as measured.
- **More vendor CLIs that log in rather than take a key.** OpenCode is the obvious fifth: it logs
  in, has hooks, runs headless. A drift probe first, as `codex` had. No ticket yet.
- **`hydra show --verdict`.** The vendor's own verdict beside the record's, as one command; today
  it is `docs/demo/verdict.py`. No ticket yet.
- **OpenTelemetry from the binary.** Today `hydra-otel`, a separate binary, converts a record to
  OTLP JSON files.
