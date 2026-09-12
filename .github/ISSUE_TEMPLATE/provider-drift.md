---
name: Provider drift
about: A vendor CLI changed something Hydra relied on
labels: drift
---

**Vendor and version** (`claude --version`, `cursor-agent --version` or `agy --version`):

**Hydra commit** (`git rev-parse --short HEAD`):

**Run id** and the lines of the record that changed (`hydra show <id> --events`):

**What Hydra did with it** (refused the launch, recorded a wrong outcome, missed a tool call):
