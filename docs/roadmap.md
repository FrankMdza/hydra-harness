# Roadmap

What is open, in the order it is likely to land. Each line names the ticket that carries it, so the
state is read from the ticket and not from this page.

- **Linux.** Builds 375 and 376 are open. Builds 318a and 318b put a Linux launch under
  bubblewrap, builds 364 and 366 give every mount a source, and build 370 measured launches in
  Docker with git working inside. Build 375 wraps the provider probes, which run unwrapped on both
  platforms. Build 376 adds `--unshare-pid`, so a run's leftover processes die with bwrap. Both
  come before a recorded run leaves macOS. The design is map 119.
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
