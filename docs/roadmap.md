# Roadmap

What is open, in the order it is likely to land. Each line names the ticket that carries it, so the
state is read from the ticket and not from this page.

- **Linux.** Builds 315 and 316 are resolved: Hydra compiles on Linux, runs CI on `ubuntu-latest`,
  and stamps `host` and `sandbox_kind` on every record. Build 318, the bubblewrap backend, is what
  remains before a Linux launch is sandboxed. Until then a Linux launch is hook-only, and recorded
  runs stay on macOS. The design is map 119.
- **Codex under `read-only`.** Routed since build 48 under `standard` and `permissive`, on the
  judge panel since 61, writing through `apply_patch` since 69. It has no read tool, so map 63
  refuses `read-only` until a bounded shell read exists.
- **A gated interactive session on `codex`, `cursor-agent` and `agy`.** Build 130 is open, and
  `integrations/herdr/interactive-gate-providers.txt` lists `claude` alone as measured.
- **More vendor CLIs that log in rather than take a key.** OpenCode is the obvious fifth: it logs
  in, has hooks, runs headless. A drift probe first, as `codex` had. No ticket yet.
- **The vendor's verdict beside the record's, as one command.** Build 335 took `--verdict` for the
  puppets goal-log verdict, so this needs another flag name. Today it is `docs/demo/verdict.py`.
  No ticket yet.
- **OpenTelemetry from the binary.** Today `hydra-otel`, a separate binary, converts a record to
  OTLP JSON files.
